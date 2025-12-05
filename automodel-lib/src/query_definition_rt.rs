/// Structures for holding complete query analysis results from Phase 1
/// This separates query analysis (DB interaction) from code generation
use crate::query_definition::QueryDefinition;
use crate::types_extractor::{ConstraintInfo, QueryTypeInfo};

/// Complete analyzed query information ready for code generation
/// This struct contains all information needed to generate code without database access
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QueryDefinitionRuntime {
    /// Original query definition from SQL file
    pub definition: QueryDefinition,

    /// Type information (input/output types, parsed SQL with conditionals)
    pub type_info: QueryTypeInfo,

    /// Whether this query is a mutation (INSERT/UPDATE/DELETE)
    /// Determined by running EXPLAIN - if EXPLAIN fails, assume mutation
    pub is_mutation: bool,

    /// Constraint information for mutation queries
    /// Empty for read-only queries
    pub constraints: Vec<ConstraintInfo>,

    /// Query execution plan analysis results (for ensure_indexes feature)
    pub performance_analysis: Option<PerformanceAnalysis>,

    /// All query variants (for conditional queries)
    /// First variant is the base query, subsequent variants include conditional blocks
    pub query_variants: Vec<String>,

    /// Clean SQL with named parameters removed (converted to positional)
    pub converted_sql: String,

    /// Parameter names extracted from SQL in order
    pub param_names: Vec<String>,
}

/// Performance analysis results from EXPLAIN
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerformanceAnalysis {
    /// Whether the query uses sequential scans
    pub has_sequential_scan: bool,

    /// Tables that are being sequentially scanned
    pub sequential_scan_tables: Vec<String>,

    /// Other performance warnings
    pub warnings: Vec<String>,

    /// Full query execution plan from EXPLAIN
    pub query_plan: Option<String>,
}

impl QueryDefinitionRuntime {
    /// Create a new query definition runtime
    pub fn new(
        definition: QueryDefinition,
        type_info: QueryTypeInfo,
        is_mutation: bool,
        constraints: Vec<ConstraintInfo>,
        performance_analysis: Option<PerformanceAnalysis>,
        query_variants: Vec<String>,
        converted_sql: String,
        param_names: Vec<String>,
    ) -> Self {
        Self {
            definition,
            type_info,
            is_mutation,
            constraints,
            performance_analysis,
            query_variants,
            converted_sql,
            param_names,
        }
    }

    /// Get the module this query belongs to
    pub fn module(&self) -> &str {
        &self.definition.module
    }
}
