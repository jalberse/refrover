
use core::fmt;
use std::{fmt::{Display, Formatter}, str::FromStr};

use diesel::{backend::Backend, deserialize::{self, FromSql, FromSqlRow}, expression::AsExpression, serialize::{IsNull, ToSql}, sql_types::Text, sqlite::Sqlite};

// We use the uuid crate, but need to wrap it in our own struct to implement ToSql.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, AsExpression, FromSqlRow, serde::Serialize, serde::Deserialize)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct UUID(uuid::Uuid);

impl From<UUID> for uuid::Uuid {
  fn from(s: UUID) -> Self {
      s.0
  }
}

impl From<uuid::Uuid> for UUID {
  fn from(s: uuid::Uuid) -> Self {
      UUID(s)
  }
}

impl From<String> for UUID {
  fn from(s: String) -> Self {
      uuid::Uuid::from_str(&s).map(UUID).unwrap()
  }
}

impl Display for UUID {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

impl<B: Backend> FromSql<Text, B> for UUID
where
    String: FromSql<Text, B>,
{
    fn from_sql(bytes: <B as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = String::from_sql(bytes)?;
        uuid::Uuid::from_str(&value.as_str())
            .map(UUID)
            .map_err(|e| e.into())
    }
}

impl ToSql<Text, Sqlite> for UUID
where
  String: ToSql<Text, Sqlite>,
{
  fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> diesel::serialize::Result {
    out.set_value(self.0.to_string());
    Ok(IsNull::No)
  }
}
