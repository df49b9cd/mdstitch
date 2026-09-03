use super::*;

// ===========================================================================
// Custom handler support
// ===========================================================================

#[test]
fn custom_handler_runs() {
    struct UpperHandler;
    impl StitchHandler for UpperHandler {
        fn handle<'a>(&self, text: &'a str) -> Cow<'a, str> {
            if text.contains("UPPER") {
                Cow::Borrowed(text)
            } else {
                Cow::Owned(text.to_uppercase())
            }
        }
        fn name(&self) -> &str {
            "upper"
        }
        fn priority(&self) -> i32 {
            200 // runs after all built-ins
        }
    }

    let opts = StitchOptions::default().handler(Box::new(UpperHandler));
    let result = stitch("hello **world", &opts);
    // Built-in bold handler closes **, then custom handler uppercases.
    assert_eq!(result.as_ref(), "HELLO **WORLD**");
}

#[test]
fn custom_handler_priority_before_builtin() {
    struct PrependHandler;
    impl StitchHandler for PrependHandler {
        fn handle<'a>(&self, text: &'a str) -> Cow<'a, str> {
            if text.starts_with("PREFIX: ") {
                Cow::Borrowed(text)
            } else {
                Cow::Owned(format!("PREFIX: {}", text))
            }
        }
        fn name(&self) -> &str {
            "prepend"
        }
        fn priority(&self) -> i32 {
            -1 // runs before all built-ins
        }
    }

    let opts = StitchOptions::default()
        .bold(false) // disable bold so we can test just the prepend
        .handler(Box::new(PrependHandler));
    let result = stitch("hello", &opts);
    assert_eq!(result.as_ref(), "PREFIX: hello");
}
