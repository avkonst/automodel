use crate::config::{
    ConditionsType, ExpectedResult, ParametersType, QueryDefinition, QueryTelemetryConfig,
    TelemetryLevel,
};
use std::collections::HashMap;

/// Builder for creating query definitions programmatically
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    name: String,
    sql: String,
    sql_file: Option<String>,
    description: Option<String>,
    module: Option<String>,
    expect: ExpectedResult,
    types: HashMap<String, String>,
    telemetry: Option<QueryTelemetryConfig>,
    ensure_indexes: Option<bool>,
    multiunzip: Option<bool>,
    conditions_type: Option<ConditionsType>,
    parameters_type: Option<ParametersType>,
    return_type: Option<String>,
    error_type: Option<String>,
}

impl QueryBuilder {
    /// Create a new query builder with required fields
    pub fn new(name: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
            sql_file: None,
            description: None,
            module: None,
            expect: ExpectedResult::ExactlyOne,
            types: HashMap::new(),
            telemetry: None,
            ensure_indexes: None,
            multiunzip: None,
            conditions_type: None,
            parameters_type: None,
            return_type: None,
            error_type: None,
        }
    }

    /// Create a new query builder with SQL loaded from a file at compile time
    ///
    /// The file path is relative to the project root (where Cargo.toml is located).
    /// The SQL file will be read at build time and embedded using `include_str!` macro.
    ///
    /// # Example
    /// ```no_run
    /// use automodel::QueryBuilder;
    ///
    /// let query = QueryBuilder::from_file("get_user", "queries/get_user.sql");
    /// ```
    pub fn from_file(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql: String::new(),
            sql_file: Some(path.into()),
            description: None,
            module: None,
            expect: ExpectedResult::ExactlyOne,
            types: HashMap::new(),
            telemetry: None,
            ensure_indexes: None,
            multiunzip: None,
            conditions_type: None,
            parameters_type: None,
            return_type: None,
            error_type: None,
        }
    }

    /// Set the SQL from a file path (uses include_str! at compile time)
    ///
    /// The file path is relative to the project root (where Cargo.toml is located).
    ///
    /// # Example
    /// ```no_run
    /// use automodel::QueryBuilder;
    ///
    /// let query = QueryBuilder::new("get_user", "")
    ///     .sql_from_file("queries/get_user.sql");
    /// ```
    pub fn sql_from_file(mut self, path: impl Into<String>) -> Self {
        self.sql_file = Some(path.into());
        self
    }

    /// Set the query description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the module name for code generation
    pub fn module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Set the expected result type
    pub fn expect(mut self, expect: ExpectedResult) -> Self {
        self.expect = expect;
        self
    }

    /// Expect exactly one result (default)
    pub fn expect_one(mut self) -> Self {
        self.expect = ExpectedResult::ExactlyOne;
        self
    }

    /// Expect zero or one result
    pub fn expect_optional(mut self) -> Self {
        self.expect = ExpectedResult::PossibleOne;
        self
    }

    /// Expect at least one result
    pub fn expect_at_least_one(mut self) -> Self {
        self.expect = ExpectedResult::AtLeastOne;
        self
    }

    /// Expect zero or more results
    pub fn expect_multiple(mut self) -> Self {
        self.expect = ExpectedResult::Multiple;
        self
    }

    /// Add a custom type mapping for a field
    pub fn map_type(mut self, field: impl Into<String>, rust_type: impl Into<String>) -> Self {
        self.types.insert(field.into(), rust_type.into());
        self
    }

    /// Add multiple type mappings
    pub fn map_types(mut self, types: HashMap<String, String>) -> Self {
        self.types.extend(types);
        self
    }

    /// Enable telemetry with specified level
    pub fn telemetry(mut self, level: TelemetryLevel) -> Self {
        let telemetry = self.telemetry.get_or_insert_with(Default::default);
        telemetry.level = Some(level);
        self
    }

    /// Include SQL in telemetry spans
    pub fn include_sql(mut self, include: bool) -> Self {
        let telemetry = self.telemetry.get_or_insert_with(Default::default);
        telemetry.include_sql = Some(include);
        self
    }

    /// Include specific parameters in telemetry
    pub fn include_params(mut self, params: Vec<String>) -> Self {
        let telemetry = self.telemetry.get_or_insert_with(Default::default);
        telemetry.include_params = Some(params);
        self
    }

    /// Enable query performance analysis
    pub fn ensure_indexes(mut self, enable: bool) -> Self {
        self.ensure_indexes = Some(enable);
        self
    }

    /// Enable multiunzip for batch inserts
    pub fn multiunzip(mut self, enable: bool) -> Self {
        self.multiunzip = Some(enable);
        self
    }

    /// Enable conditions_type (diff-based conditional parameters)
    pub fn conditions_type(mut self, enable: bool) -> Self {
        self.conditions_type = Some(ConditionsType::Enabled(enable));
        self
    }

    /// Use named conditions_type struct
    pub fn conditions_type_named(mut self, name: impl Into<String>) -> Self {
        self.conditions_type = Some(ConditionsType::Named(name.into()));
        self
    }

    /// Enable parameters_type (structured parameters)
    pub fn parameters_type(mut self, enable: bool) -> Self {
        self.parameters_type = Some(ParametersType::Enabled(enable));
        self
    }

    /// Use named parameters_type struct
    pub fn parameters_type_named(mut self, name: impl Into<String>) -> Self {
        self.parameters_type = Some(ParametersType::Named(name.into()));
        self
    }

    /// Set custom return type name
    pub fn return_type(mut self, name: impl Into<String>) -> Self {
        self.return_type = Some(name.into());
        self
    }

    /// Set custom error type name
    pub fn error_type(mut self, name: impl Into<String>) -> Self {
        self.error_type = Some(name.into());
        self
    }

    /// Build the QueryDefinition
    pub fn build(self) -> QueryDefinition {
        // If sql_file is specified, read the file content at build time
        let sql = if let Some(ref path) = self.sql_file {
            std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read SQL file '{}': {}", path, e))
        } else {
            self.sql
        };

        QueryDefinition {
            name: self.name,
            sql,
            description: self.description,
            module: self.module,
            expect: self.expect,
            types: if self.types.is_empty() {
                None
            } else {
                Some(self.types)
            },
            telemetry: self.telemetry,
            ensure_indexes: self.ensure_indexes,
            multiunzip: self.multiunzip,
            conditions_type: self.conditions_type,
            parameters_type: self.parameters_type,
            return_type: self.return_type,
            error_type: self.error_type,
        }
    }
}

/// Builder for creating AutoModel configuration
#[derive(Debug, Clone, Default)]
pub struct AutoModelBuilder {
    queries: Vec<QueryDefinition>,
    default_telemetry_level: Option<TelemetryLevel>,
    default_include_sql: Option<bool>,
    default_ensure_indexes: Option<bool>,
    default_module: Option<String>,
}

impl AutoModelBuilder {
    /// Create a new AutoModelBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a query using a QueryBuilder
    pub fn query(mut self, query_builder: QueryBuilder) -> Self {
        self.queries.push(query_builder.build());
        self
    }

    /// Add a pre-built QueryDefinition
    pub fn add_query(mut self, query: QueryDefinition) -> Self {
        self.queries.push(query);
        self
    }

    /// Add multiple queries
    pub fn queries(mut self, queries: Vec<QueryDefinition>) -> Self {
        self.queries.extend(queries);
        self
    }

    /// Set default telemetry level for all queries
    pub fn default_telemetry(mut self, level: TelemetryLevel) -> Self {
        self.default_telemetry_level = Some(level);
        self
    }

    /// Set default SQL inclusion for telemetry
    pub fn default_include_sql(mut self, include: bool) -> Self {
        self.default_include_sql = Some(include);
        self
    }

    /// Set default index analysis setting
    pub fn default_ensure_indexes(mut self, enable: bool) -> Self {
        self.default_ensure_indexes = Some(enable);
        self
    }

    /// Set default module for queries without explicit module
    pub fn default_module(mut self, module: impl Into<String>) -> Self {
        self.default_module = Some(module.into());
        self
    }

    /// Get the queries (internal use)
    pub(crate) fn get_queries(self) -> Vec<QueryDefinition> {
        let mut queries = self.queries;

        // Apply defaults to queries that don't have them set
        for query in &mut queries {
            if query.telemetry.is_none() {
                query.telemetry = Some(QueryTelemetryConfig::default());
            }
            if let Some(telemetry) = query.telemetry.as_mut() {
                if telemetry.level.is_none() {
                    telemetry.level = self.default_telemetry_level;
                }
                if telemetry.include_sql.is_none() {
                    telemetry.include_sql = self.default_include_sql;
                }
            }
            if query.ensure_indexes.is_none() {
                query.ensure_indexes = self.default_ensure_indexes;
            }
            if query.module.is_none() {
                query.module = self.default_module.clone();
            }
        }

        queries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_basic() {
        let query = QueryBuilder::new("get_user", "SELECT * FROM users WHERE id = ${id}")
            .description("Get user by ID")
            .module("users")
            .expect_one()
            .build();

        assert_eq!(query.name, "get_user");
        assert_eq!(query.sql, "SELECT * FROM users WHERE id = ${id}");
        assert_eq!(query.description, Some("Get user by ID".to_string()));
        assert_eq!(query.module, Some("users".to_string()));
        assert_eq!(query.expect, ExpectedResult::ExactlyOne);
    }

    #[test]
    fn test_query_builder_with_types() {
        let query = QueryBuilder::new(
            "insert_user",
            "INSERT INTO users (profile) VALUES (${profile})",
        )
        .map_type("profile", "crate::models::UserProfile")
        .expect_one()
        .build();

        assert!(query.types.is_some());
        let types = query.types.unwrap();
        assert_eq!(
            types.get("profile"),
            Some(&"crate::models::UserProfile".to_string())
        );
    }

    #[test]
    fn test_automodel_builder() {
        let builder = AutoModelBuilder::new()
            .default_telemetry(TelemetryLevel::Debug)
            .default_include_sql(true)
            .query(
                QueryBuilder::new("get_user", "SELECT * FROM users WHERE id = ${id}")
                    .module("users"),
            )
            .query(
                QueryBuilder::new("insert_user", "INSERT INTO users (name) VALUES (${name})")
                    .module("users")
                    .telemetry(TelemetryLevel::Trace),
            );

        let queries = builder.get_queries();
        assert_eq!(queries.len(), 2);

        // First query should have default telemetry level
        assert_eq!(
            queries[0].telemetry.as_ref().unwrap().level,
            Some(TelemetryLevel::Debug)
        );

        // Second query should have its own telemetry level
        assert_eq!(
            queries[1].telemetry.as_ref().unwrap().level,
            Some(TelemetryLevel::Trace)
        );
    }
}
