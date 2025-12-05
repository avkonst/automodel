/// Structures for holding complete query analysis results from Phase 1
/// This separates query analysis (DB interaction) from code generation
use crate::query_definition::QueryDefinition;
use crate::types_extractor::{ConstraintInfo, QueryTypeInfo};

/// Pre-computed EXPLAIN query parameters for a single query variant
#[derive(Debug, Clone)]
pub struct ExplainParams {
    /// The EXPLAIN SQL query with special params inlined and remaining params renumbered
    pub explain_sql: String,
    /// Indices of special parameters that were inlined in explain_sql
    /// Used to identify which dummy params to bind for remaining parameters
    pub special_params: Vec<usize>,
}

/// Result of analyzing a query with EXPLAIN
/// Contains mutation detection, performance analysis, constraints, and pre-computed EXPLAIN params
#[derive(Debug, Clone)]
pub struct QueryAnalysisResult {
    /// Whether this query is a mutation (INSERT/UPDATE/DELETE)
    pub is_mutation: bool,

    /// Performance analysis results (only for queries with ensure_indexes enabled)
    pub performance_analysis: Option<PerformanceAnalysis>,

    /// Constraint information for mutation queries
    pub constraints: Vec<ConstraintInfo>,

    /// Pre-computed EXPLAIN query parameters for each variant
    /// None if variant has no parameters
    pub explain_params: Vec<Option<ExplainParams>>,
}

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

    /// Pre-computed EXPLAIN query parameters for each variant
    /// None if variant has no parameters
    pub explain_params: Vec<Option<ExplainParams>>,
}

/// Performance analysis results from EXPLAIN
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerformanceAnalysis {
    /// Query name for warnings
    pub query_name: String,

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
        explain_params: Vec<Option<ExplainParams>>,
    ) -> Self {
        Self {
            definition,
            type_info,
            is_mutation,
            constraints,
            performance_analysis,
            explain_params,
        }
    }

    /// Get the module this query belongs to
    pub fn module(&self) -> &str {
        &self.definition.module
    }
}
