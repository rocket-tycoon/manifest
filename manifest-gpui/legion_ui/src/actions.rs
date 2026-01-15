use gpui::*;

actions!(legion, [OpenProject, Quit]);

/// Action to open a specific recent project by ID
#[derive(Clone, PartialEq, Debug)]
pub struct OpenRecentProject {
    pub project_id: String,
}

impl gpui::Action for OpenRecentProject {
    fn name(&self) -> &'static str {
        "legion::OpenRecentProject"
    }

    fn name_for_type() -> &'static str
    where
        Self: Sized,
    {
        "legion::OpenRecentProject"
    }

    fn build(value: serde_json::Value) -> anyhow::Result<Box<dyn Action>>
    where
        Self: Sized,
    {
        let project_id = value
            .get("project_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(Box::new(Self { project_id }))
    }

    fn partial_eq(&self, other: &dyn Action) -> bool {
        if let Some(other) = other.boxed_clone().as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
}
