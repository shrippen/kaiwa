//! The demo profile (build feature `demo`, never in a package): two weeks of a beginner's
//! history for screenshots.

use kaiwa_core::clock::Day;
use kaiwa_core::difficulty::{Difficulty, Regulator};
use kaiwa_core::learner::{LearnerModel, Skill};
use kaiwa_core::progress::DayRecord;
use kaiwa_core::srs::{Grade, Item, ItemKind, Source};
use kaiwa_store::{Learning, Profile};

use std::sync::Arc;

use kaiwa_ai::ChatProvider;
use kaiwa_ai::mock::MockProvider;

use crate::{Backend, fixtures, now};

/// Two weeks of a beginner's history: units 1 and 2 done, unit 3 under way, words to review,
/// a few gaps. For screenshots and the demo.
pub(crate) fn seed(st: &Backend) -> anyhow::Result<()> {
    let now = now();
    let today = now.day();
    let profile = Profile {
        name: "Mara".into(),
        placed: true,
        ui_lang: if std::env::var("KAIWA_LANG").as_deref() == Ok("en") {
            "en".into()
        } else {
            "de".into()
        },
        ..Default::default()
    };
    st.store.set_profile(&profile)?;
    let model = LearnerModel::placed(&[
        (Skill::Listening, "n5_low"),
        (Skill::Speaking, "a0"),
        (Skill::Fluency, "a0"),
        (Skill::ReadingKana, "a0"),
        (Skill::ReadingKanji, "none"),
        (Skill::Register, "casual_only"),
    ]);
    let mut learning = Learning {
        regulator: Regulator::new(Difficulty::for_learner(&model)),
        model,
        ..Default::default()
    };
    learning.model.errors.insert(
        "particle".into(),
        kaiwa_core::learner::ErrorStat {
            corrected: 4,
            rate: 0.3,
        },
    );
    learning.model.errors.insert(
        "counter".into(),
        kaiwa_core::learner::ErrorStat {
            corrected: 3,
            rate: 0.22,
        },
    );
    for s in [0.74, 0.81, 0.86, 0.79, 0.9, 0.83, 0.78] {
        let model = learning.model.clone();
        learning.regulator.after_session(s, &model);
    }
    for s in ["n5-01-aisatsu", "n5-02-jikoshoukai"] {
        learning.course.scenarios_done.insert(s.into());
        learning.course.completions.insert(s.into(), 2);
    }
    learning
        .course
        .completions
        .insert("n5-04-konbini".into(), 1);
    st.store.set_learning(&learning)?;
    // Days: practised on 13 of the last 14 (one gap, covered by a freeze), two minutes today.
    let minutes = [6, 5, 0, 7, 5, 12, 6, 5, 6, 5, 8, 5, 6, 7];
    // Where the days started from (/LF604/): listening and fluency have grown since.
    let mut levels_at_start: std::collections::BTreeMap<Skill, f64> = kaiwa_core::learner::SKILLS
        .iter()
        .map(|s| (*s, learning.model.level(*s)))
        .collect();
    for (skill, less) in [(Skill::Listening, 0.3), (Skill::Fluency, 0.2)] {
        if let Some(l) = levels_at_start.get_mut(&skill) {
            *l -= less;
        }
    }
    for (i, m) in minutes.iter().enumerate() {
        if *m == 0 {
            continue;
        }
        let d = today.plus(i as i64 - minutes.len() as i64);
        st.store.put_day(
            d,
            &DayRecord {
                speaking_secs: m * 60,
                scenarios_done: u32::from(*m >= 6),
                reviews_done: 4,
                xp: m * 18,
                levels_at_start: levels_at_start.clone(),
                ..Default::default()
            },
        )?;
    }
    st.store.put_day(
        today,
        &DayRecord {
            speaking_secs: 130,
            xp: 40,
            ..Default::default()
        },
    )?;
    let words = [
        (
            "コーヒーが好きです",
            ItemKind::Pattern,
            Some("particle"),
            "n5-02",
        ),
        (
            "ビールを二つください",
            ItemKind::Pattern,
            Some("counter"),
            "n5-04",
        ),
        ("おはようございます", ItemKind::Phrase, None, "n5-01"),
        ("天気", ItemKind::Word, None, "n5-01"),
        ("仕事", ItemKind::Word, None, "n5-02"),
        ("袋", ItemKind::Word, None, "n5-04"),
        ("温める", ItemKind::Word, None, "n5-04"),
        ("いくら", ItemKind::Word, None, "n5-04"),
    ];
    // The talk the café correction came from, with the recorded answers (never a tutor).
    let played = {
        let sc = st
            .scenarios
            .get("n5-02-jikoshoukai")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("scenario n5-02"))?;
        let mock: Arc<dyn ChatProvider> = Arc::new(
            fixtures("n5-02-jikoshoukai")
                .and_then(|f| MockProvider::from_json(f).ok())
                .unwrap_or_default(),
        );
        let mut s =
            kaiwa_core::Session::new(sc, mock, kaiwa_core::Amount::Little, &profile.ui_lang);
        for line in ["仕事はプロデューサーです。", "コーヒーを好きです。"] {
            s.turn(line, kaiwa_core::Input::Spoken)?;
        }
        s.summary()
    };
    for (n, (w, kind, corr, unit)) in words.iter().enumerate() {
        let added = Day(today.0 - 9 + n as i64)
            .start(now.offset_min)
            .plus_secs(9 * 3600);
        let source = match corr {
            Some(k) => Source::Correction {
                kind: (*k).to_string(),
            },
            None => Source::NewWord,
        };
        let mut item = Item::new(*w, *kind, source, added);
        item.unit = Some((*unit).to_string());
        // A correction from the played n5-02 talk comes back as a mini conversation: Tanaka's
        // line and the old words are the recorded ones (/LF230/).
        if corr.is_some() {
            let key = kaiwa_core::norm(w);
            item.cue = played.cues.get(&key).cloned();
            item.original = played.mistakes.get(&key).cloned();
        }
        item.review(Grade::Good, added.plus_secs(86_400));
        if n % 3 == 0 {
            item.due = today;
        }
        st.store.put_item(&item)?;
    }
    // Talks of the last weeks with their corrections, for the statistics (/LF610/): particles
    // fade, counters stay. Recorded answers, so the corrections are the real ones.
    let n502 = st
        .scenarios
        .get("n5-02-jikoshoukai")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("scenario n5-02"))?;
    for (days_ago, particles, counters) in [
        (70, 3, 1),
        (56, 3, 1),
        (42, 2, 1),
        (28, 1, 1),
        (21, 1, 2),
        (1, 0, 2),
    ] {
        // Always the recorded answers here, never a configured tutor.
        let mock: Arc<dyn ChatProvider> = Arc::new(
            fixtures("n5-02-jikoshoukai")
                .and_then(|f| MockProvider::from_json(f).ok())
                .unwrap_or_default()
                .with_fallback("そうですか。"),
        );
        let mut s = kaiwa_core::Session::new(
            n502.clone(),
            mock,
            kaiwa_core::Amount::Little,
            &profile.ui_lang,
        );
        s.turn(
            "マラです。よろしくおねがいします。",
            kaiwa_core::Input::Spoken,
        )?;
        for _ in 0..particles {
            s.turn("コーヒーを好きです。", kaiwa_core::Input::Spoken)?;
        }
        for _ in 0..counters {
            s.turn("ビール二ください。", kaiwa_core::Input::Spoken)?;
        }
        let started = now.plus_secs(-days_ago * 86_400 - 3600);
        st.store
            .put_finished_session(&s.snapshot(), started, started.plus_secs(420), 0.8)?;
        st.track_grammar(&s, started)?;
    }
    Ok(())
}
