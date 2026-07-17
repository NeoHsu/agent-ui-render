use std::collections::BTreeMap;

use serde_json::Value;

use crate::domain::{Column, Dataset, Primitive};

use super::{
    CompactColumnMeta, TYPE_CODE_DICT_PREFIX, TYPE_CODE_STRING, codes::is_primitive,
    normalize_type_code,
};

pub(super) fn normalize_compact_dataset(
    compact_columns: Vec<Value>,
    raw_data: Vec<Value>,
    is_column_major: bool,
    dictionaries: &BTreeMap<String, Vec<String>>,
) -> (Dataset, Vec<CompactColumnMeta>) {
    let metas: Vec<CompactColumnMeta> = compact_columns
        .iter()
        .enumerate()
        .map(compact_column_meta)
        .collect();
    let columns: Vec<Column> = compact_columns
        .iter()
        .enumerate()
        .map(normalize_compact_column)
        .collect();
    let raw_rows = if is_column_major {
        transpose_columns(raw_data)
    } else {
        raw_data
    };
    let rows = raw_rows
        .iter()
        .filter_map(Value::as_array)
        .map(|row| {
            columns
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    normalize_cell(
                        row.get(index).cloned().unwrap_or(Value::Null),
                        metas
                            .get(index)
                            .map_or(TYPE_CODE_STRING, |meta| meta.type_code.as_str()),
                        dictionaries,
                    )
                })
                .collect()
        })
        .collect();
    (Dataset { columns, rows }, metas)
}

fn compact_column_meta((index, value): (usize, &Value)) -> CompactColumnMeta {
    let tuple = value.as_array();
    CompactColumnMeta {
        key: tuple
            .and_then(|items| items.first())
            .and_then(Value::as_str)
            .map_or_else(|| format!("column_{}", index + 1), ToOwned::to_owned),
        type_code: tuple
            .and_then(|items| items.get(1))
            .and_then(Value::as_str)
            .unwrap_or(TYPE_CODE_STRING)
            .to_owned(),
    }
}

fn normalize_compact_column((index, value): (usize, &Value)) -> Column {
    let tuple = value.as_array();
    let key = tuple
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .map_or_else(|| format!("column_{}", index + 1), ToOwned::to_owned);
    let type_code = tuple
        .and_then(|items| items.get(1))
        .and_then(Value::as_str)
        .unwrap_or(TYPE_CODE_STRING);
    let unit = tuple
        .and_then(|items| items.get(2))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let label = tuple
        .and_then(|items| items.get(3))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| Some(titleize_key(&key)));
    Column {
        key,
        label,
        column_type: Some(normalize_type_code(type_code)),
        unit,
        description: None,
    }
}

pub(super) fn read_dictionaries(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, entries)| {
            let strings: Vec<String> = entries
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect();
            Some((key.clone(), strings))
        })
        .collect()
}

fn transpose_columns(columns: Vec<Value>) -> Vec<Value> {
    let column_arrays: Vec<Vec<Value>> = columns
        .into_iter()
        .filter_map(|item| item.as_array().cloned())
        .collect();
    let row_count = column_arrays.first().map_or(0, Vec::len);
    (0..row_count)
        .map(|row_index| {
            Value::Array(
                column_arrays
                    .iter()
                    .map(|column| column.get(row_index).cloned().unwrap_or(Value::Null))
                    .collect(),
            )
        })
        .collect()
}

fn normalize_cell(
    value: Value,
    type_code: &str,
    dictionaries: &BTreeMap<String, Vec<String>>,
) -> Primitive {
    if let Some(dict_id) = type_code.strip_prefix(TYPE_CODE_DICT_PREFIX)
        && let Some(index) = value.as_u64()
    {
        return dictionaries
            .get(dict_id)
            .and_then(|entries| entries.get(index as usize))
            .map_or(Value::Null, |text| Value::String(text.clone()));
    }
    if is_primitive(&value) {
        value
    } else {
        Value::Null
    }
}

fn titleize_key(key: &str) -> String {
    key.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
