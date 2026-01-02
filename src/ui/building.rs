#[macro_export]
macro_rules! ui_builder {
    // ===== ohne Kinder =====
    (
        $name:expr,
        $element:expr
    ) => {
        $element.wrap($name)
    };

    // ===== ohne Kinder =====
    (
        $element:expr
    ) => {
        $element.wrap("")
    };

    // ===== Kinder als Slice =====
    (
        $name:expr,
        $element:expr,
        $children:expr
    ) => {
        let mut v = Vec::new();
        v.extend_from_slice($children);
        $element.wrap_childs($name, n)
    };

    // ===== Kinder als Slice =====
    (
        $element:expr,
        $children:expr
    ) => {
        let mut v = Vec::new();
        v.extend_from_slice(&$children);
        $element.wrap_childs("", v)
    };

    // ===== Inline Kinder =====
    (
        $name:expr,
        $element:expr,
        {
            $($child:expr),* $(,)?
        }
    ) => {{
        $element.wrap_childs(
            $name,
            $element,
            vec![$($child),*],
        )
    }};
}
