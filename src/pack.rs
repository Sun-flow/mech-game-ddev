use crate::unit::UnitKind;

#[derive(Clone, Debug)]
pub struct PackDef {
    pub kind: UnitKind,
    pub rows: u8,
    pub cols: u8,
    pub cost: u32,
    pub name: &'static str,
}

impl PackDef {
    pub fn count(&self) -> u8 {
        self.rows * self.cols
    }
}

pub fn all_packs() -> &'static [PackDef] {
    &[
        // T1 - 100 gold
        PackDef {
            kind: UnitKind::Chaff,
            rows: 3,
            cols: 6,
            cost: 100,
            name: "Chaff",
        },
        PackDef {
            kind: UnitKind::Scout,
            rows: 2,
            cols: 3,
            cost: 100,
            name: "Scouts",
        },
        // T2 - 200 gold
        PackDef {
            kind: UnitKind::Striker,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Strikers",
        },
        PackDef {
            kind: UnitKind::Bruiser,
            rows: 1,
            cols: 2,
            cost: 200,
            name: "Bruisers",
        },
        PackDef {
            kind: UnitKind::Sentinel,
            rows: 1,
            cols: 2,
            cost: 200,
            name: "Sentinels",
        },
        PackDef {
            kind: UnitKind::Ranger,
            rows: 1,
            cols: 3,
            cost: 200,
            name: "Rangers",
        },
        // T3 - 300 gold
        PackDef {
            kind: UnitKind::Artillery,
            rows: 1,
            cols: 2,
            cost: 300,
            name: "Artillery",
        },
    ]
}
