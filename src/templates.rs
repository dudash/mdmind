#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Product,
    Feature,
    Prompts,
    Backlog,
    Writing,
}

impl TemplateKind {
    pub fn all() -> &'static [Self] {
        &[
            Self::Product,
            Self::Feature,
            Self::Prompts,
            Self::Backlog,
            Self::Writing,
        ]
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "product" => Some(Self::Product),
            "feature" => Some(Self::Feature),
            "prompts" => Some(Self::Prompts),
            "backlog" => Some(Self::Backlog),
            "writing" => Some(Self::Writing),
            _ => None,
        }
    }

    pub fn names() -> &'static [&'static str] {
        &["product", "feature", "prompts", "backlog", "writing"]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Feature => "feature",
            Self::Prompts => "prompts",
            Self::Backlog => "backlog",
            Self::Writing => "writing",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Product => "Product roadmap, requirements, milestones, and supporting links.",
            Self::Feature => "Feature spec with goals, scope, risks, delivery, and open questions.",
            Self::Prompts => "Prompt library, evaluation notes, and operational prompt workflows.",
            Self::Backlog => "Backlog-oriented planning with now, next, later, and blocked work.",
            Self::Writing => "Writing and research structure for stories, notes, and chapter work.",
        }
    }

    pub fn default_file_name(self) -> &'static str {
        match self {
            Self::Product => "product.md",
            Self::Feature => "feature.md",
            Self::Prompts => "prompts.md",
            Self::Backlog => "backlog.md",
            Self::Writing => "writing.md",
        }
    }

    pub fn file_contents(self) -> &'static str {
        match self {
            Self::Product => include_str!("../templates/product.md"),
            Self::Feature => include_str!("../templates/feature.md"),
            Self::Prompts => include_str!("../templates/prompts.md"),
            Self::Backlog => include_str!("../templates/backlog.md"),
            Self::Writing => include_str!("../templates/writing.md"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::has_errors;
    use crate::parser::parse_document;
    use crate::validate::validate_document;

    use super::TemplateKind;

    #[test]
    fn every_template_parses_and_validates_cleanly() {
        for template in [
            TemplateKind::Product,
            TemplateKind::Feature,
            TemplateKind::Prompts,
            TemplateKind::Backlog,
            TemplateKind::Writing,
        ] {
            let parsed = parse_document(template.file_contents());
            assert!(
                !has_errors(&parsed.diagnostics),
                "parser diagnostics for template {:?}: {:?}",
                template,
                parsed.diagnostics
            );
            let validation = validate_document(&parsed.document);
            assert!(
                !has_errors(&validation),
                "validation diagnostics for template {:?}: {:?}",
                template,
                validation
            );
        }
    }
}
