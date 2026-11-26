// Dummy placeholder module
use sysinfo::Networks;

pub fn dummy_noop() {
    let mut n = Networks::new_with_refreshed_list();
    n.refresh(true);
    for (_name, data) in &n {
        for ip in data.ip_networks() {
            let _ = ip.addr;
        }
    }
}
