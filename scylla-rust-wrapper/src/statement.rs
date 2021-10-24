use crate::argconv::*;
use crate::cass_error::CassError;
use crate::collection::CassCollection;
use crate::inet::CassInet;
use crate::query_result::CassResult;
use crate::types::*;
use crate::uuid::CassUuid;
use scylla::frame::response::result::CqlValue;
use scylla::frame::response::result::CqlValue::*;
use scylla::frame::types::Consistency;
use scylla::frame::value::MaybeUnset;
use scylla::frame::value::MaybeUnset::{Set, Unset};
use scylla::query::Query;
use scylla::statement::prepared_statement::PreparedStatement;
use scylla::Bytes;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;

#[derive(Clone)]
pub enum Statement {
    Simple(Query),
    // Arc is needed, because PreparedStatement is passed by reference to session.execute
    Prepared(Arc<PreparedStatement>),
}

pub struct CassStatement {
    pub statement: Statement,
    pub bound_values: Vec<MaybeUnset<Option<CqlValue>>>,
    pub paging_state: Option<Bytes>,
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_new(
    query: *const c_char,
    parameter_count: size_t,
) -> *mut CassStatement {
    let query_str = match ptr_to_cstr(query) {
        Some(v) => v,
        None => return std::ptr::null_mut(),
    };
    let query_length = query_str.len();

    cass_statement_new_n(query, query_length as size_t, parameter_count)
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_new_n(
    query: *const c_char,
    query_length: size_t,
    parameter_count: size_t,
) -> *mut CassStatement {
    let query_str = match ptr_to_cstr_n(query, query_length) {
        Some(v) => v,
        None => return std::ptr::null_mut(),
    };

    let mut query = Query::new(query_str.to_string());

    // Set Cpp Driver default configuration for queries:
    query.disable_paging();
    query.set_consistency(Consistency::One);

    Box::into_raw(Box::new(CassStatement {
        statement: Statement::Simple(query),
        bound_values: vec![Unset; parameter_count as usize],
        paging_state: None,
    }))
}

// TODO: Bind methods currently not implemented:
// cass_statement_bind_decimal
//
// cass_statement_bind_duration - DURATION not implemented in Rust Driver
//
// (methods requiring implementing cpp driver data structures)
// cass_statement_bind_user_type
// cass_statement_bind_collection
// cass_statement_bind_custom
// cass_statement_bind_custom_n
// cass_statement_bind_tuple
//
// Variants of all methods with by_name, by_name_n

unsafe fn cass_statement_bind_maybe_unset(
    statement_raw: *mut CassStatement,
    index: size_t,
    value: MaybeUnset<Option<CqlValue>>,
) -> CassError {
    let statement = ptr_to_ref_mut(statement_raw);

    if index as usize >= statement.bound_values.len() {
        CassError::CASS_ERROR_LIB_INDEX_OUT_OF_BOUNDS
    } else {
        statement.bound_values[index as usize] = value;
        CassError::CASS_OK
    }
}

unsafe fn cass_statement_bind_cql_value(
    statement: *mut CassStatement,
    index: size_t,
    value: CqlValue,
) -> CassError {
    cass_statement_bind_maybe_unset(statement, index, Set(Some(value)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_null(
    statement: *mut CassStatement,
    index: size_t,
) -> CassError {
    cass_statement_bind_maybe_unset(statement, index, Set(None))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_int8(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_int8_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, TinyInt(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_int16(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_int16_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, SmallInt(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_int32(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_int32_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, Int(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_uint32(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_uint32_t,
) -> CassError {
    // cass_statement_bind_uint32 is only used to set a DATE.
    cass_statement_bind_cql_value(statement, index, Date(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_int64(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_int64_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, BigInt(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_float(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_float_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, Float(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_double(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_double_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, Double(value))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_bool(
    statement: *mut CassStatement,
    index: size_t,
    value: cass_bool_t,
) -> CassError {
    cass_statement_bind_cql_value(statement, index, Boolean(value != 0))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_string(
    statement: *mut CassStatement,
    index: size_t,
    value: *const c_char,
) -> CassError {
    let value_str = ptr_to_cstr(value).unwrap();
    let value_length = value_str.len();

    cass_statement_bind_string_n(statement, index, value, value_length as size_t)
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_string_n(
    statement: *mut CassStatement,
    index: size_t,
    value: *const c_char,
    value_length: size_t,
) -> CassError {
    // TODO: Error handling
    let value_string = ptr_to_cstr_n(value, value_length).unwrap().to_string();
    cass_statement_bind_cql_value(statement, index, Text(value_string))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_bytes(
    statement: *mut CassStatement,
    index: size_t,
    value: *const cass_byte_t,
    value_size: size_t,
) -> CassError {
    let value_vec = std::slice::from_raw_parts(value, value_size as usize).to_vec();
    cass_statement_bind_cql_value(statement, index, Blob(value_vec))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_inet(
    statement: *mut CassStatement,
    index: size_t,
    value: CassInet,
) -> CassError {
    // FIXME: implement _by_name and _by_name_n variants
    cass_statement_bind_cql_value(statement, index, Inet(value.into()))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_uuid(
    statement: *mut CassStatement,
    index: size_t,
    value: CassUuid,
) -> CassError {
    // FIXME: implement _by_name and _by_name_n variants
    cass_statement_bind_cql_value(statement, index, CqlValue::Uuid(value.into()))
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_bind_collection(
    statement: *mut CassStatement,
    index: size_t,
    collection_raw: *const CassCollection,
) -> CassError {
    // FIXME: implement _by_name and _by_name_n variants
    let collection = ptr_to_ref(collection_raw).clone();
    cass_statement_bind_cql_value(statement, index, collection.into())
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_set_tracing(
    statement_raw: *mut CassStatement,
    enabled: cass_bool_t,
) -> CassError {
    match &mut ptr_to_ref_mut(statement_raw).statement {
        Statement::Simple(inner) => inner.set_tracing(enabled != 0),
        Statement::Prepared(inner) => Arc::make_mut(inner).set_tracing(enabled != 0),
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_set_paging_size(
    statement_raw: *mut CassStatement,
    page_size: c_int,
) -> CassError {
    // TODO: validate page_size
    match &mut ptr_to_ref_mut(statement_raw).statement {
        Statement::Simple(inner) => {
            if page_size == -1 {
                inner.disable_paging()
            } else {
                inner.set_page_size(page_size)
            }
        }
        Statement::Prepared(inner) => {
            if page_size == -1 {
                Arc::make_mut(inner).disable_paging()
            } else {
                Arc::make_mut(inner).set_page_size(page_size)
            }
        }
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_set_paging_state(
    statement: *mut CassStatement,
    result: *const CassResult,
) -> CassError {
    let statement = ptr_to_ref_mut(statement);
    let result = ptr_to_ref(result);

    statement.paging_state = result.paging_state.clone();
    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_set_is_idempotent(
    statement_raw: *mut CassStatement,
    is_idempotent: cass_bool_t,
) -> CassError {
    match &mut ptr_to_ref_mut(statement_raw).statement {
        Statement::Simple(inner) => inner.set_is_idempotent(is_idempotent != 0),
        Statement::Prepared(inner) => Arc::make_mut(inner).set_is_idempotent(is_idempotent != 0),
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_free(statement_raw: *mut CassStatement) {
    free_boxed(statement_raw);
}
