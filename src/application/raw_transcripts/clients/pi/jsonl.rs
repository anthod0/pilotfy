use std::fs;

use serde_json::Value;

use crate::error::{Error, Result};

use super::super::super::{
    RawTranscriptParser, ResolvedAgentBinding, TimelineItemDetailPage, TimelineItemDetailRequest,
    TimelinePage, TimelinePageRequest,
};
use super::{
    mapping::pi_entry_to_items,
    refs::{CursorPosition, decode_pi_content_ref, decode_pi_cursor, encode_pi_cursor},
};

#[derive(Debug, Clone, Default)]
pub struct PiJsonlParser;

impl PiJsonlParser {
    pub fn new() -> Self {
        Self
    }
}

impl RawTranscriptParser for PiJsonlParser {
    fn client_type(&self) -> &'static str {
        "pi"
    }

    fn format(&self) -> &'static str {
        "pi-jsonl"
    }

    fn timeline_page(&self, request: TimelinePageRequest) -> Result<TimelinePage> {
        if request.source.client_type != self.client_type()
            || request.source.format != self.format()
        {
            return Err(Error::CapabilityUnavailable(format!(
                "unsupported source {}/{} for pi jsonl parser",
                request.source.client_type, request.source.format
            )));
        }

        let source_id = source_id(&request.source);
        let cursor = decode_pi_cursor(request.cursor.as_deref(), &request.source.id)?;
        let bytes = fs::read(&request.source.path).map_err(|err| {
            Error::CapabilityUnavailable(format!(
                "source_unavailable: raw source {} is unavailable: {err}",
                request.source.path.display()
            ))
        })?;

        if cursor.offset > bytes.len() {
            return Err(Error::Domain(format!(
                "cursor_invalid: offset {} exceeds source length {}",
                cursor.offset,
                bytes.len()
            )));
        }

        let limit = request.limit.max(1);
        let mut items = Vec::new();
        let mut offset = 0usize;
        let mut next_position = CursorPosition {
            offset: cursor.offset,
            block_index: cursor.block_index,
        };
        let mut stopped_due_limit = false;

        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            let line_start = offset;
            let line_end = offset + line.len();
            offset = line_end;

            if line_end <= cursor.offset {
                continue;
            }
            if line_start < cursor.offset {
                continue;
            }

            let text = std::str::from_utf8(line)
                .map_err(|err| Error::Domain(format!("pi jsonl source is not utf-8: {err}")))?
                .trim_end_matches(['\r', '\n']);
            if text.trim().is_empty() {
                next_position = CursorPosition {
                    offset: line_end,
                    block_index: 0,
                };
                continue;
            }
            let entry: Value = serde_json::from_str(text)?;
            let produced = pi_entry_to_items(&entry, &request.source.id, line_start, line_end);
            let start_block = if line_start == cursor.offset {
                cursor.block_index
            } else {
                0
            };

            for (idx, item) in produced.into_iter().enumerate().skip(start_block) {
                if items.len() == limit {
                    stopped_due_limit = true;
                    next_position = CursorPosition {
                        offset: line_start,
                        block_index: idx,
                    };
                    break;
                }
                items.push(item);
                next_position = CursorPosition {
                    offset: line_start,
                    block_index: idx + 1,
                };
            }

            if stopped_due_limit {
                break;
            }

            next_position = CursorPosition {
                offset: line_end,
                block_index: 0,
            };
        }

        let has_unread_bytes = next_position.offset < bytes.len();
        let has_more = stopped_due_limit || has_unread_bytes;
        let cursor_token = encode_pi_cursor(&request.source.id, next_position);

        Ok(TimelinePage {
            session_id: request.session_id,
            binding_id: request.source.id,
            items,
            next_cursor: Some(cursor_token.clone()),
            tail_cursor: Some(cursor_token),
            has_more,
            is_tail: !has_more,
            source_id,
        })
    }

    fn timeline_item_detail(
        &self,
        request: TimelineItemDetailRequest,
    ) -> Result<TimelineItemDetailPage> {
        let detail_ref = decode_pi_content_ref(&request.content_ref, &request.source.id)?;
        let bytes = fs::read(&request.source.path)?;
        if detail_ref.start > detail_ref.end || detail_ref.end > bytes.len() {
            return Err(Error::Domain(
                "content_ref_invalid: byte range outside source".to_string(),
            ));
        }
        let line = std::str::from_utf8(&bytes[detail_ref.start..detail_ref.end])
            .map_err(|err| {
                Error::Domain(format!("content_ref_invalid: source is not utf-8: {err}"))
            })?
            .trim_end_matches(['\r', '\n']);
        let entry: Value = serde_json::from_str(line)?;
        let text = match detail_ref.kind.as_str() {
            "assistant" | "thinking" | "tool_call" => entry
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(|content| content.get(detail_ref.block_index))
                .cloned()
                .unwrap_or(Value::Null),
            _ => entry.clone(),
        };
        let text = serde_json::to_string_pretty(&text)?;
        Ok(TimelineItemDetailPage {
            binding_id: request.source.id,
            content_ref: request.content_ref,
            content_type: "application/json".to_string(),
            size_bytes: text.len(),
            text,
        })
    }
}

fn source_id(source: &ResolvedAgentBinding) -> String {
    format!("{}:{}", source.client_type, source.path.display())
}
