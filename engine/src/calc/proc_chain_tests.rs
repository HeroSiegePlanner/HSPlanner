// Proc-made objects roll the skill's other procs: InheritModifiers leaves out only the roll that
// made them, so a chain runs through distinct procs.
use super::{chain_hits, ChainProc};

fn proc(hits: f64) -> ChainProc {
    ChainProc {
        hits,
        ..ChainProc::default()
    }
}

fn procs(hits: &[f64]) -> Vec<ChainProc> {
    hits.iter().map(|&hits| proc(hits)).collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn two_procs_roll_each_other_once() {
    let (a, b) = (2.0, 3.0);
    let chained = chain_hits(&procs(&[a, b]), 0b11);
    assert!(close(chained, a + b + 2.0 * a * b));
}

#[test]
fn three_procs_chain_through_every_order() {
    let (a, b, c) = (0.5, 2.0, 3.0);
    let pairs = 2.0 * (a * b + a * c + b * c);
    let expected = a + b + c + pairs + 6.0 * a * b * c;
    assert!(close(chain_hits(&procs(&[a, b, c]), 0b111), expected));
}

#[test]
fn a_proc_only_the_skill_rolls_starts_chains_but_never_continues_them() {
    let (aid, flames) = (0.5, 2.0);
    let aid_proc = ChainProc {
        base_only: true,
        ..proc(aid)
    };
    let chained = chain_hits(&[aid_proc, proc(flames)], 0b11);
    assert!(close(chained, aid + flames + aid * flames));
}

#[test]
fn objects_that_roll_some_procs_pass_only_those_on() {
    let (a, b, c) = (0.5, 2.0, 3.0);
    let rolled = chain_hits(&procs(&[a, b, c]), 0b011);
    assert!(close(rolled, a + b + 2.0 * a * b));
}

#[test]
fn a_proc_whose_objects_roll_nothing_ends_every_chain_it_joins() {
    let (nova, fragments) = (0.5, 2.0);
    let silent = ChainProc {
        rolls: Some(0),
        ..proc(nova)
    };
    let chained = chain_hits(&[silent, proc(fragments)], 0b11);
    assert!(close(chained, nova + fragments + fragments * nova));
}

#[test]
fn an_absolute_proc_deals_its_share_on_every_hit_that_rolls_it() {
    // Two rolling hits of a quarter share each, then a nova whose damage the cast fixed.
    let (fragments, nova) = (0.5, 3.0);
    let split = ChainProc {
        count: Some(2.0),
        ..proc(fragments)
    };
    let fixed = ChainProc {
        absolute: true,
        ..proc(nova)
    };
    let chained = chain_hits(&[split, fixed], 0b11);
    let expected = fragments + 2.0 * nova + nova + nova * fragments;
    assert!(close(chained, expected));
}
