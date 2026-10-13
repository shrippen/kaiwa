# Grammar catalogue

One file per level (`n5.json`, …), format in ADR-009: every grammar point a unit of
`data/curriculum/` names, with an ASCII id, the label the unit uses, an explanation in Kaiwa's
own words (de, en) and examples in furigana markup. The level comes from the file and must be the
level of the unit that introduces the point (`Curriculum::check_grammar`, tested in
`kaiwa-core`).

The explanations are written for Kaiwa. Other guides (Tae Kim: CC BY-NC-SA) serve only as
orientation for order and structure; no text is taken over.
