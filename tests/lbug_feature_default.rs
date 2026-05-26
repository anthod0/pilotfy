#[test]
fn lbug_feature_is_enabled_by_default() {
    assert!(
        cfg!(feature = "lbug"),
        "llmparty's default build must include the mandatory lbug feature"
    );
}
