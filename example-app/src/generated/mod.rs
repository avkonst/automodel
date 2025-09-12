pub mod admin;
pub mod setup;
pub mod users;

use serde::{Serialize, Deserialize};
use tokio_postgres::types::{FromSql, ToSql, Type};
use std::error::Error;

/// Test query with JSON parameter
/// Generated from SQL: SELECT ${test_data}::jsonb as test_data
pub async fn test_json_query(client: &tokio_postgres::Client, test_data: serde_json::Value) -> Result<Option<crate::models::TestData>, tokio_postgres::Error> {
    let stmt = client.prepare("SELECT $1::jsonb as test_data").await?;
    let rows = client.query(&stmt, &[&test_data]).await?;
    let extracted_value = if let Some(row) = rows.into_iter().next() {
        row.get::<_, Option<JsonWrapper<crate::models::TestData>>>(0).map(|wrapper| wrapper.into_inner())
    } else {
        None
    };
    Ok(extracted_value)
}


// JSON wrapper for custom types that implement Serialize/Deserialize
struct JsonWrapper<T>(T);

impl<T> JsonWrapper<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    fn new(value: T) -> Self {
        Self(value)
    }
    
    fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromSql<'_> for JsonWrapper<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::Value::from_sql(ty, raw)?;
        let value = T::deserialize(json_value)?;
        Ok(JsonWrapper(value))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }
}

impl<T> ToSql for JsonWrapper<T>
where
    T: Serialize + std::fmt::Debug,
{
    fn to_sql(&self, ty: &Type, out: &mut bytes::BytesMut) -> Result<tokio_postgres::types::IsNull, Box<dyn Error + Sync + Send>> {
        let json_value = serde_json::to_value(&self.0)?;
        json_value.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::JSON | Type::JSONB)
    }

    tokio_postgres::types::to_sql_checked!();
}

impl<T> std::fmt::Debug for JsonWrapper<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JsonWrapper").field(&self.0).finish()
    }
}
