use crate::argconv::*;
use crate::cass_error::{self, CassError};
use crate::types::*;
use core::time::Duration;
use scylla::SessionBuilder;
use std::os::raw::{c_char, c_int, c_uint};
use scylla::load_balancing::{DcAwareRoundRobinPolicy, RoundRobinPolicy, TokenAwarePolicy, LoadBalancingPolicy, ChildLoadBalancingPolicy};
use std::sync::Arc;

enum ChildBalancingPolicy {
    RoundRobinPolicy(RoundRobinPolicy),
    DcAwareRoundRobinPolicy(DcAwareRoundRobinPolicy)
}

pub struct CassCluster {
    pub session_builder: SessionBuilder,

    pub contact_points: Vec<String>,
    pub port: u16,

    pub child_load_balancing_policy: ChildBalancingPolicy,
    pub token_aware_policy_enabled: bool,
}

pub fn build_session_builder(cluster: &CassCluster) -> SessionBuilder {
    let known_nodes: Vec<_> = cluster
        .contact_points
        .clone()
        .into_iter()
        .map(|cp| format!("{}:{}", cp, cluster.port))
        .collect();

    let load_balancing: Arc<dyn LoadBalancingPolicy> = match &cluster.child_load_balancing_policy {
        ChildBalancingPolicy::RoundRobinPolicy(policy) => {
            if cluster.token_aware_policy_enabled {
                let inner: Box<dyn ChildLoadBalancingPolicy> = Box::new(policy.clone());
                Arc::new(TokenAwarePolicy::new(inner))
            } else {
                Arc::new(policy)
            }
        }
        ChildBalancingPolicy::DcAwareRoundRobinPolicy(policy) => {
            if cluster.token_aware_policy_enabled {
                Arc::new(TokenAwarePolicy::new(Box::new(policy)))
            } else {
                Arc::new(policy)
            }
        }
    };

    cluster.session_builder.clone().known_nodes(&known_nodes).load_balancing(load_balancing)
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_new() -> *mut CassCluster {
    Box::into_raw(Box::new(CassCluster {
        session_builder: SessionBuilder::new(),
        port: 9042,
        contact_points: Vec::new(),
        // Per DataStax documentation: Without additional configuration the C/C++ driver 
        // defaults to using Datacenter-aware load balancing with token-aware routing.
        child_load_balancing_policy: ChildBalancingPolicy::DcAwareRoundRobinPolicy(DcAwareRoundRobinPolicy::new("".to_string())),
        token_aware_policy_enabled: true,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_free(cluster: *mut CassCluster) {
    free_boxed(cluster);
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_contact_points(
    cluster: *mut CassCluster,
    contact_points: *const c_char,
) -> CassError {
    let contact_points_str = ptr_to_cstr(contact_points).unwrap();
    let contact_points_length = contact_points_str.len();

    cass_cluster_set_contact_points_n(cluster, contact_points, contact_points_length as size_t)
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_contact_points_n(
    cluster: *mut CassCluster,
    contact_points: *const c_char,
    contact_points_length: size_t,
) -> CassError {
    match cluster_set_contact_points(cluster, contact_points, contact_points_length) {
        Ok(()) => cass_error::OK,
        Err(err) => err,
    }
}

unsafe fn cluster_set_contact_points(
    cluster_raw: *mut CassCluster,
    contact_points_raw: *const c_char,
    contact_points_length: size_t,
) -> Result<(), CassError> {
    // FIXME: validate contact points (whether they are valid inets)

    let cluster = ptr_to_ref_mut(cluster_raw);
    let mut contact_points = ptr_to_cstr_n(contact_points_raw, contact_points_length)
        .ok_or(cass_error::LIB_BAD_PARAMS)?
        .split(',')
        .peekable();

    if contact_points.peek().is_none() {
        // If cass_cluster_set_contact_points() is called with empty
        // set of contact points, the contact points should be cleared.
        cluster.contact_points.clear();
        return Ok(());
    }

    // cass_cluster_set_contact_points() will append
    // in subsequent calls, not overwrite.
    cluster
        .contact_points
        .extend(contact_points.map(|cp| cp.to_string()));
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_tcp_nodelay(
    cluster_raw: *mut CassCluster,
    enabled: cass_bool_t,
) {
    let cluster = ptr_to_ref_mut(cluster_raw);
    cluster.session_builder.config.tcp_nodelay = enabled != 0;
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_connect_timeout(
    cluster_raw: *mut CassCluster,
    timeout_ms: c_uint,
) {
    let cluster = ptr_to_ref_mut(cluster_raw);
    cluster.session_builder.config.connect_timeout = Duration::from_millis(timeout_ms.into());
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_port(
    cluster_raw: *mut CassCluster,
    port: c_int,
) -> CassError {
    let cluster = ptr_to_ref_mut(cluster_raw);
    cluster.port = port as u16; // FIXME: validate port number
    cass_error::OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_credentials(
    cluster: *mut CassCluster,
    username: *const c_char,
    password: *const c_char,
) {
    let username_str = ptr_to_cstr(username).unwrap();
    let username_length = username_str.len();

    let password_str = ptr_to_cstr(password).unwrap();
    let password_length = password_str.len();

    cass_cluster_set_credentials_n(
        cluster,
        username,
        username_length as size_t,
        password,
        password_length as size_t,
    )
}

#[no_mangle]
pub unsafe extern "C" fn cass_cluster_set_credentials_n(
    cluster_raw: *mut CassCluster,
    username_raw: *const c_char,
    username_length: size_t,
    password_raw: *const c_char,
    password_length: size_t,
) {
    let username = ptr_to_cstr_n(username_raw, username_length).unwrap();
    let password = ptr_to_cstr_n(password_raw, password_length).unwrap();

    let cluster = ptr_to_ref_mut(cluster_raw);
    cluster.session_builder.config.auth_username = Some(username.to_string());
    cluster.session_builder.config.auth_password = Some(password.to_string());
}
