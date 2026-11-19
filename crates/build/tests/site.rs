use hsplanner_build::site;

#[test]
fn build_id_reads_site_links_in_the_forms_people_paste() {
    for link in [
        "https://hsplanner.app/b/PfGpnVEmjB",
        "  https://hsplanner.app/b/PfGpnVEmjB\n",
        "http://hsplanner.app/b/PfGpnVEmjB",
        "hsplanner.app/b/PfGpnVEmjB",
        "https://www.hsplanner.app/b/PfGpnVEmjB",
        "https://hsplanner.app/b/PfGpnVEmjB/",
        "https://hsplanner.app/b/PfGpnVEmjB/raw",
        "https://hsplanner.app/b/PfGpnVEmjB?gear=1&tree=0#gear",
        "HTTPS://HSPlanner.app/b/PfGpnVEmjB",
    ] {
        assert_eq!(site::build_id(link), Some("PfGpnVEmjB"), "{link}");
    }
}

#[test]
fn build_id_refuses_other_hosts_and_malformed_ids() {
    for input in [
        "",
        "N4IgbiBcDMA0IGcB2",
        "https://evil.example/b/PfGpnVEmjB",
        "https://hsplanner.app.evil.example/b/PfGpnVEmjB",
        "https://evil.example/?x=hsplanner.app/b/PfGpnVEmjB",
        "https://hsplanner.app@evil.example/b/PfGpnVEmjB",
        "https://hsplanner.app/b/short",
        "https://hsplanner.app/b/PfGpnVEmjBtoolong",
        "https://hsplanner.app/b/PfGpn..mjB",
        "https://hsplanner.app/b/../../etc",
        "https://hsplanner.app/PfGpnVEmjB",
        "https://hsplanner.app/b/",
    ] {
        assert_eq!(site::build_id(input), None, "{input}");
    }
}

#[test]
fn addresses_stay_on_the_site() {
    assert_eq!(site::API_URL, "https://hsplanner.app/api/pastes");
    assert_eq!(
        site::raw_url("PfGpnVEmjB"),
        "https://hsplanner.app/b/PfGpnVEmjB/raw"
    );
}

#[test]
fn shared_link_is_built_from_the_replied_id_not_from_the_replied_address() {
    let reply = br#"{"success":true,"data":{"id":"PfGpnVEmjB","url":"https://evil.example/x"},"error":null}"#;

    assert_eq!(
        site::shared_link(reply).as_deref(),
        Some("https://hsplanner.app/b/PfGpnVEmjB")
    );
}

#[test]
fn shared_link_refuses_replies_without_a_proper_id() {
    for reply in [
        &b"not json"[..],
        br#"{"success":false,"data":null,"error":"no"}"#,
        br#"{"success":true,"data":{"id":"../../etc"}}"#,
        br#"{"success":true,"data":{"id":7}}"#,
        br#"[]"#,
    ] {
        assert_eq!(site::shared_link(reply), None);
    }
}
