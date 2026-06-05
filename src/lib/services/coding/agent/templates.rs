use super::types::{CodeTemplate, ProjectType, TemplateVariable, VariableType};
use anyhow::Result;
use std::collections::HashMap;

/// Template manager for code generation
pub struct TemplateManager {
    templates: Vec<CodeTemplate>,
}

impl TemplateManager {
    pub fn new() -> Self {
        Self {
            templates: Self::initialize_default_templates(),
        }
    }

    /// Initialize default code templates
    fn initialize_default_templates() -> Vec<CodeTemplate> {
        vec![
            // Rust function template
            CodeTemplate {
                name: "rust_function".to_string(),
                description: "Basic Rust function template".to_string(),
                language: ProjectType::Rust,
                template_content: r#"/// {description}
pub fn {function_name}({parameters}) -> {return_type} {
    {body}
}"#
                .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "function_name".to_string(),
                        description: "Name of the function".to_string(),
                        default_value: Some("new_function".to_string()),
                        required: true,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "parameters".to_string(),
                        description: "Function parameters".to_string(),
                        default_value: Some("".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "return_type".to_string(),
                        description: "Return type".to_string(),
                        default_value: Some("()".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "description".to_string(),
                        description: "Function description".to_string(),
                        default_value: Some("TODO: Add description".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "body".to_string(),
                        description: "Function body".to_string(),
                        default_value: Some("todo!()".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                ],
                dependencies: vec![],
                use_cases: vec![
                    "Creating new functions".to_string(),
                    "Implementing methods".to_string(),
                ],
            },
            // Rust struct template
            CodeTemplate {
                name: "rust_struct".to_string(),
                description: "Basic Rust struct template".to_string(),
                language: ProjectType::Rust,
                template_content: r#"/// {description}
#[derive({derives})]
pub struct {struct_name} {
{fields}
}"#
                .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "struct_name".to_string(),
                        description: "Name of the struct".to_string(),
                        default_value: Some("NewStruct".to_string()),
                        required: true,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "description".to_string(),
                        description: "Struct description".to_string(),
                        default_value: Some("TODO: Add description".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "derives".to_string(),
                        description: "Derive macros".to_string(),
                        default_value: Some("Debug, Clone".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "fields".to_string(),
                        description: "Struct fields".to_string(),
                        default_value: Some("    // TODO: Add fields".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                ],
                dependencies: vec![],
                use_cases: vec![
                    "Data structures".to_string(),
                    "Configuration objects".to_string(),
                ],
            },
            // Rust error enum template
            CodeTemplate {
                name: "rust_error".to_string(),
                description: "Rust error enum with thiserror".to_string(),
                language: ProjectType::Rust,
                template_content: r#"use thiserror::Error;

#[derive(Error, Debug)]
pub enum {error_name} {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Custom error: {message}")]
    Custom { message: String },

    #[error("Validation failed: {field}")]
    Validation { field: String },
}

pub type Result<T> = std::result::Result<T, {error_name}>;
"#
                .to_string(),
                variables: vec![TemplateVariable {
                    name: "error_name".to_string(),
                    description: "Name of the error enum".to_string(),
                    default_value: Some("AppError".to_string()),
                    required: true,
                    variable_type: VariableType::String,
                }],
                dependencies: vec!["thiserror".to_string()],
                use_cases: vec![
                    "Error handling".to_string(),
                    "Library development".to_string(),
                ],
            },
            // JavaScript/TypeScript async function template
            CodeTemplate {
                name: "js_async_function".to_string(),
                description: "JavaScript/TypeScript async function template".to_string(),
                language: ProjectType::JavaScript,
                template_content: r#"/**
 * {description}
 * @param {{params_doc}}
 * @returns {{return_doc}}
 */
async function {function_name}({parameters}) {
    try {
        {body}
    } catch (error) {
        console.error(`Error in {function_name}:`, error);
        throw error;
    }
}
"#
                .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "function_name".to_string(),
                        description: "Name of the function".to_string(),
                        default_value: Some("newAsyncFunction".to_string()),
                        required: true,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "parameters".to_string(),
                        description: "Function parameters".to_string(),
                        default_value: Some("".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "description".to_string(),
                        description: "Function description".to_string(),
                        default_value: Some("TODO: Add description".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "params_doc".to_string(),
                        description: "Parameter documentation".to_string(),
                        default_value: Some("Object".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "return_doc".to_string(),
                        description: "Return value documentation".to_string(),
                        default_value: Some("Promise<void>".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "body".to_string(),
                        description: "Function body".to_string(),
                        default_value: Some("        // TODO: Implement function".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                ],
                dependencies: vec![],
                use_cases: vec!["Async operations".to_string(), "API calls".to_string()],
            },
            // Python class template
            CodeTemplate {
                name: "python_class".to_string(),
                description: "Python class template".to_string(),
                language: ProjectType::Python,
                template_content: r#"class {class_name}:
    """
    {description}
    """
    
    def __init__(self{init_params}):
        """
        Initialize {class_name}.
        
        Args:
            {init_args_doc}
        """
        {init_body}
    
    def {method_name}(self{method_params}):
        """
        {method_description}
        
        Args:
            {method_args_doc}
            
        Returns:
            {method_return_doc}
        """
        {method_body}
"#
                .to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "class_name".to_string(),
                        description: "Name of the class".to_string(),
                        default_value: Some("NewClass".to_string()),
                        required: true,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "description".to_string(),
                        description: "Class description".to_string(),
                        default_value: Some("TODO: Add class description".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "init_params".to_string(),
                        description: "Constructor parameters".to_string(),
                        default_value: Some("".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "init_args_doc".to_string(),
                        description: "Constructor arguments documentation".to_string(),
                        default_value: Some("None".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "init_body".to_string(),
                        description: "Constructor body".to_string(),
                        default_value: Some("        pass".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_name".to_string(),
                        description: "Example method name".to_string(),
                        default_value: Some("example_method".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_params".to_string(),
                        description: "Method parameters".to_string(),
                        default_value: Some("".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_description".to_string(),
                        description: "Method description".to_string(),
                        default_value: Some("TODO: Add method description".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_args_doc".to_string(),
                        description: "Method arguments documentation".to_string(),
                        default_value: Some("None".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_return_doc".to_string(),
                        description: "Method return documentation".to_string(),
                        default_value: Some("None".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                    TemplateVariable {
                        name: "method_body".to_string(),
                        description: "Method body".to_string(),
                        default_value: Some("        pass".to_string()),
                        required: false,
                        variable_type: VariableType::String,
                    },
                ],
                dependencies: vec![],
                use_cases: vec![
                    "Object-oriented programming".to_string(),
                    "Class creation".to_string(),
                ],
            },
        ]
    }

    /// Get available code templates for a project type
    pub fn get_templates(&self, project_type: Option<ProjectType>) -> Vec<&CodeTemplate> {
        match project_type {
            Some(pt) => self.templates.iter().filter(|t| t.language == pt).collect(),
            None => self.templates.iter().collect(),
        }
    }

    /// Get a specific template by name
    pub fn get_template(&self, name: &str) -> Option<&CodeTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }

    /// Generate code from template with variable substitution
    pub fn generate_from_template(
        &self,
        template_name: &str,
        variables: HashMap<String, String>,
    ) -> Result<String> {
        let template = self
            .get_template(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        let mut content = template.template_content.clone();

        // Substitute variables
        for variable in &template.variables {
            let value = variables
                .get(&variable.name)
                .or(variable.default_value.as_ref())
                .ok_or_else(|| {
                    anyhow::anyhow!("Required variable '{}' not provided", variable.name)
                })?;

            let placeholder = format!("{{{}}}", variable.name);
            content = content.replace(&placeholder, value);
        }

        Ok(content)
    }

    /// Add a custom code template
    pub fn add_template(&mut self, template: CodeTemplate) {
        // Remove existing template with the same name
        self.templates.retain(|t| t.name != template.name);
        self.templates.push(template);
    }

    /// Remove a template by name
    pub fn remove_template(&mut self, name: &str) -> bool {
        let initial_len = self.templates.len();
        self.templates.retain(|t| t.name != name);
        self.templates.len() < initial_len
    }

    /// List all template names
    pub fn list_template_names(&self) -> Vec<String> {
        self.templates.iter().map(|t| t.name.clone()).collect()
    }

    /// Get template by language
    pub fn get_templates_by_language(&self, language: ProjectType) -> Vec<&CodeTemplate> {
        self.templates
            .iter()
            .filter(|t| t.language == language)
            .collect()
    }

    /// Search templates by use case
    pub fn search_templates_by_use_case(&self, use_case: &str) -> Vec<&CodeTemplate> {
        self.templates
            .iter()
            .filter(|t| t.use_cases.iter().any(|uc| uc.contains(use_case)))
            .collect()
    }

    /// Validate template variables
    pub fn validate_template_variables(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let template = self
            .get_template(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        let mut missing_required = Vec::new();

        for variable in &template.variables {
            if variable.required
                && !variables.contains_key(&variable.name)
                && variable.default_value.is_none()
            {
                missing_required.push(variable.name.clone());
            }
        }

        if missing_required.is_empty() {
            Ok(vec![])
        } else {
            Ok(missing_required)
        }
    }

    /// Get template dependencies
    pub fn get_template_dependencies(&self, template_name: &str) -> Option<Vec<String>> {
        self.get_template(template_name)
            .map(|t| t.dependencies.clone())
    }

    /// Create a template from existing code
    pub fn create_template_from_code(
        &mut self,
        name: String,
        description: String,
        language: ProjectType,
        code: &str,
        variables: Vec<TemplateVariable>,
    ) -> Result<()> {
        let template = CodeTemplate {
            name,
            description,
            language,
            template_content: code.to_string(),
            variables,
            dependencies: vec![],
            use_cases: vec!["Custom template".to_string()],
        };

        self.add_template(template);
        Ok(())
    }

    /// Export templates to JSON
    pub fn export_templates(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.templates)
            .map_err(|e| anyhow::anyhow!("Failed to export templates: {}", e))
    }

    /// Import templates from JSON
    pub fn import_templates(&mut self, json: &str) -> Result<usize> {
        let imported_templates: Vec<CodeTemplate> = serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to parse templates JSON: {}", e))?;

        let count = imported_templates.len();
        for template in imported_templates {
            self.add_template(template);
        }

        Ok(count)
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}
