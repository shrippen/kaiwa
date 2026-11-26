//! The names of the settings, as the front ends pass them. One list: the setting commands,
//! the journal and the per-device rule all read it, so a name exists once.

/// Declares [`Key`] with the text each name has at the API.
macro_rules! keys {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// One known setting.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum Key {
            $($variant),+
        }

        impl Key {
            /// The key for `name`; `None` when no setting has that name.
            pub(crate) fn parse(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// The name the settings and the journal use.
            pub(crate) fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }
    };
}

keys! {
    Goal => "goal",
    Gamification => "gamification",
    Feedback => "feedback",
    Script => "script",
    EndPause => "endPause",
    Style => "style",
    UiLang => "uiLang",
    Reminder => "reminder",
    UnloadAfter => "unloadAfter",
    CostLimit => "costLimit",
    MoraHints => "moraHints",
    HandsFree => "handsFree",
    TextScale => "textScale",
    KeepRecordings => "keepRecordings",
    Name => "name",
    TutorProvider => "tutorProvider",
    TutorModel => "tutorModel",
    TutorBase => "tutorBase",
    Microphone => "microphone",
}

impl Key {
    /// Settings that belong to this device, not to the profile: applied live, never
    /// journaled, so a second device keeps its own tutor and microphone. The owner decided
    /// the list on 17.11.2026 (ADR-010): tutor, microphone, unload time, hands-free, louder
    /// and clear. The last two are set by `set_sound`, not as a setting, and for the same
    /// reason not journaled either.
    pub(crate) fn per_device(self) -> bool {
        matches!(
            self,
            Self::TutorProvider
                | Self::TutorModel
                | Self::TutorBase
                | Self::Microphone
                | Self::UnloadAfter
                | Self::HandsFree
        )
    }
}

/// Puts this device's own settings ([`Key::per_device`], louder and clear) from `device`
/// into `profile`: after a fold, which only knows the profile's.
pub(crate) fn keep_device(profile: &mut kaiwa_store::Profile, device: &kaiwa_store::Profile) {
    profile.tutor = device.tutor.clone();
    profile.microphone = device.microphone.clone();
    profile.unload_after_min = device.unload_after_min;
    profile.hands_free = device.hands_free;
    profile.louder = device.louder;
    profile.clear = device.clear;
}
