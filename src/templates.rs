#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Product,
    Feature,
    Prompts,
    Backlog,
}

impl TemplateKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "product" => Some(Self::Product),
            "feature" => Some(Self::Feature),
            "prompts" => Some(Self::Prompts),
            "backlog" => Some(Self::Backlog),
            _ => None,
        }
    }

    pub fn names() -> &'static [&'static str] {
        &["product", "feature", "prompts", "backlog"]
    }

    pub fn file_contents(self) -> &'static str {
        match self {
            Self::Product => include_str!("../templates/product.md"),
            Self::Feature => include_str!("../templates/feature.md"),
            Self::Prompts => include_str!("../templates/prompts.md"),
            Self::Backlog => include_str!("../templates/backlog.md"),
        }
    }
}
