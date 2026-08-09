use rusqlite::{Connection, params_from_iter, types::Value};

use crate::{
    BackendError, IntentUnitPage, IntentUnitSummary, ListCursor, ListIntentUnits,
    sqlite::{StoredRow, classify_runtime_error, status_projection},
};

const SELECT_ROW: &str = "SELECT id, envelope_version, envelope, workflow_id, species, phase, status, revision \
     FROM intent_units";
const WORKFLOW_FILTER: &str = "workflow_id COLLATE BINARY = ?";
const SPECIES_FILTER: &str = "species COLLATE BINARY = ?";
const PHASE_FILTER: &str = "phase COLLATE BINARY = ?";
const STATUS_FILTER: &str = "status COLLATE BINARY = ?";
const CURSOR_FILTER: &str = "id COLLATE BINARY > ?";
const ORDER_AND_LIMIT: &str = " ORDER BY id COLLATE BINARY ASC LIMIT ?";

pub(crate) fn list(
    connection: &Connection,
    command: &ListIntentUnits,
) -> Result<IntentUnitPage, BackendError> {
    let mut predicates = Vec::with_capacity(5);
    let mut values = Vec::with_capacity(6);
    let filters = command.filters();

    if let Some(workflow_id) = filters.workflow_id() {
        predicates.push(WORKFLOW_FILTER);
        values.push(Value::Text(workflow_id.as_str().to_owned()));
    }
    if let Some(species) = filters.species() {
        predicates.push(SPECIES_FILTER);
        values.push(Value::Text(species.as_str().to_owned()));
    }
    if let Some(phase) = filters.phase() {
        predicates.push(PHASE_FILTER);
        values.push(Value::Text(phase.as_str().to_owned()));
    }
    if let Some(status) = filters.status() {
        predicates.push(STATUS_FILTER);
        values.push(Value::Text(status_projection(status).to_owned()));
    }
    if let Some(after) = command.after() {
        predicates.push(CURSOR_FILTER);
        values.push(Value::Text(after.to_string()));
    }

    let mut sql = String::from(SELECT_ROW);
    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    sql.push_str(ORDER_AND_LIMIT);

    let limit = command.limit().value();
    let fetch_limit = limit
        .checked_add(1)
        .expect("validated page limit plus lookahead must fit usize");
    values.push(Value::Integer(
        i64::try_from(fetch_limit).expect("page limit plus lookahead must fit SQLite INTEGER"),
    ));

    let mut statement = connection.prepare(&sql).map_err(classify_runtime_error)?;
    let candidates = statement
        .query_map(params_from_iter(values.iter()), StoredRow::from_row)
        .map_err(classify_runtime_error)?;
    let mut summaries = Vec::with_capacity(fetch_limit);
    for candidate in candidates {
        let unit = candidate
            .map_err(classify_runtime_error)?
            .into_validated_unit()?;
        summaries.push(IntentUnitSummary::from_intent_unit(&unit));
    }

    let has_more = summaries.len() > limit;
    summaries.truncate(limit);
    let next_cursor = if has_more {
        summaries
            .last()
            .map(|summary| ListCursor::from_id(summary.id()))
    } else {
        None
    };
    Ok(IntentUnitPage::new(summaries, next_cursor))
}
