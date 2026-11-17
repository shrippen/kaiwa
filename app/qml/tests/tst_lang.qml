import QtQuick
import QtTest
import "../lang.js" as Lang

// The language of the pages (lang.js): an unknown or empty one reads German, as in the
// backend. Run: tools/qml-tests.sh.
TestCase {
    name: "Lang"

    function test_the_pages_pick_by_the_code() {
        compare(Lang.pick("de", "Hallo", "Hello"), "Hallo")
        compare(Lang.pick("en", "Hallo", "Hello"), "Hello")
    }

    function test_an_unknown_or_empty_code_is_german() {
        compare(Lang.pick("", "Hallo", "Hello"), "Hallo")
        compare(Lang.pick("fr", "Hallo", "Hello"), "Hallo")
        compare(Lang.pick(undefined, "Hallo", "Hello"), "Hallo")
    }

    function test_the_choice_maps_to_the_code_and_back() {
        compare(Lang.indexOf("en"), 1)
        compare(Lang.indexOf("de"), 0)
        compare(Lang.indexOf(""), 0)
        compare(Lang.code(1), "en")
        compare(Lang.code(0), "de")
        compare(Lang.code(7), "de")
    }
}
