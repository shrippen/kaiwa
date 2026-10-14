# Dictionary excerpt (demo and tests)

`kaiwa-dict-excerpt.sqlite` holds the rows of the data package
[kaiwa-data](https://github.com/shrippen/kaiwa-data) 0.2.0 that the words of the scenarios, the
recorded answers and the QA scripts look up (1,880 of 218,908 JMdict entries, 828 JMnedict
names, the tanos levels of the words in it), in the same schema. The demo (`./start.sh demo`, through `KAIWA_DICT`) and the tests read it; the program
never contains it, and the learner downloads the full package in the settings (M-03).

**Licence:** CC BY-SA 4.0, like the package. Derived from JMdict and KANJIDIC2, property of the
Electronic Dictionary Research and Development Group (EDRDG), used in conformance with the
Group's licence (https://www.edrdg.org/edrdg/licence.html), from JMnedict (EDRDG, same licence),
from the Kanjium pitch accent data by Uros Ozvatic (https://github.com/mifunetoshiro/kanjium) and
from the JLPT lists of Jonathan Waller, tanos.co.uk (CC BY). Unchanged rows; the selection
is the change.

Rebuild after a change to the scenarios or fixtures (the full package unpacked anywhere):

```sh
cargo run -p kaiwa-lang --example excerpt -- <path>/kaiwa-dict.sqlite
```
