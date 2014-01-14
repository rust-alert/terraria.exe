use tr_core::{DamageHit, DamageType, ResistProfile, resolve_damage};

#[test]
fn true_ignores_resist() {
    let r = ResistProfile {
        kinetic: 0.1,
        ..ResistProfile::neutral()
    };
    assert!((resolve_damage(DamageHit::environmental(50.0), r) - 50.0).abs() < 1e-4);
}

#[test]
fn slime_weak_to_elemental() {
    let raw = 20.0;
    let out = resolve_damage(
        DamageHit::new(raw, DamageType::Elemental),
        ResistProfile::slime(),
    );
    assert!((out - raw * 1.35).abs() < 1e-4);
}

#[test]
fn slime_resist_toxin() {
    let out = resolve_damage(DamageHit::new(20.0, DamageType::Toxin), ResistProfile::slime());
    assert!((out - 11.0).abs() < 1e-4);
}
