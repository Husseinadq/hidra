//! The G502 X family's controls and the actions they can be given.

pub struct Control {
    pub id: &'static str,
    pub tag: &'static str,
    pub name: &'static str,
    /// Index of this button in an onboard profile's button table.
    pub slot: usize,
    /// Hotspot centre on the 300×440 mouse drawing.
    pub at: (f32, f32),
    pub default_action: &'static str,
}

pub const CONTROLS: &[Control] = &[
    Control { id: "left", tag: "L", name: "Left click", slot: 0, at: (105.0, 112.0), default_action: "lclick" },
    Control { id: "right", tag: "R", name: "Right click", slot: 1, at: (205.0, 112.0), default_action: "rclick" },
    Control { id: "wheel", tag: "M", name: "Wheel click", slot: 2, at: (155.0, 91.0), default_action: "mclick" },
    Control { id: "tiltl", tag: "TL", name: "Wheel tilt left", slot: 6, at: (122.0, 62.0), default_action: "sleft" },
    Control { id: "tiltr", tag: "TR", name: "Wheel tilt right", slot: 7, at: (188.0, 62.0), default_action: "sright" },
    Control { id: "g9", tag: "G9", name: "Profile button (G9)", slot: 8, at: (155.0, 142.0), default_action: "cycleprofile" },
    Control { id: "g8", tag: "G8", name: "Front left (G8)", slot: 9, at: (67.0, 90.0), default_action: "dpiup" },
    Control { id: "g7", tag: "G7", name: "Rear left (G7)", slot: 10, at: (67.0, 128.0), default_action: "dpidown" },
    Control { id: "g6", tag: "G6", name: "Thumb · DPI shift (G6)", slot: 4, at: (25.0, 181.0), default_action: "dpishift" },
    Control { id: "g5", tag: "G5", name: "Thumb · front (G5)", slot: 5, at: (22.0, 221.0), default_action: "forward" },
    Control { id: "g4", tag: "G4", name: "Thumb · rear (G4)", slot: 3, at: (22.0, 259.0), default_action: "back" },
];

pub struct ActionGroup {
    pub name: &'static str,
    pub actions: &'static [(&'static str, &'static str)],
}

pub const ACTION_GROUPS: &[ActionGroup] = &[
    ActionGroup {
        name: "Mouse",
        actions: &[
            ("lclick", "Left click"),
            ("rclick", "Right click"),
            ("mclick", "Middle click"),
            ("sleft", "Scroll left"),
            ("sright", "Scroll right"),
            ("none", "Do nothing"),
        ],
    },
    ActionGroup { name: "Navigation", actions: &[("back", "Back"), ("forward", "Forward"), ("nexttab", "Next tab"), ("prevtab", "Previous tab")] },
    ActionGroup {
        name: "Pointer",
        actions: &[
            ("dpiup", "DPI up"),
            ("dpidown", "DPI down"),
            ("dpishift", "DPI shift (hold)"),
            ("dpicycle", "Cycle DPI"),
            ("cycleprofile", "Next onboard profile"),
        ],
    },
    ActionGroup {
        name: "Desktop",
        actions: &[("workspaces", "Show workspaces"), ("wsnext", "Next workspace"), ("wsprev", "Previous workspace"), ("screenshot", "Screenshot")],
    },
    ActionGroup { name: "Media", actions: &[("play", "Play / pause"), ("volup", "Volume up"), ("voldown", "Volume down"), ("mute", "Mute")] },
    ActionGroup { name: "Custom", actions: &[("key", "Keyboard shortcut"), ("cmd", "Run command")] },
];

/// Actions the mouse can't store in its own memory.
pub fn needs_software(action: &str) -> bool {
    action == "cmd"
}

pub fn action_label(id: &str) -> &'static str {
    if id == "other" {
        return "Other (set elsewhere)";
    }
    ACTION_GROUPS
        .iter()
        .flat_map(|g| g.actions.iter())
        .find(|(a, _)| *a == id)
        .map(|(_, l)| *l)
        .unwrap_or("Unassigned")
}

pub fn control(id: &str) -> Option<&'static Control> {
    CONTROLS.iter().find(|c| c.id == id)
}
