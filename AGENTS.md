## DESCRIPTION
This is an application which controls the ROG Falchion TLK keyboard on MacOS.  THis keyboard seems to use proprietary drivers and command-sets.  Currently its settings are controlled via Armory Crate, which is only available on Windows.

## Details
* GUI App: Yes
    * Web App: Maybe
* CLI: Yes
* Language: Rust.
* Ideally this will work cross-platform, but MacOS is the initial target
* There must be a user-friendly users guide for this application (USER-GUIDE.md).  This must be kept as up to date as possible.  It tells users how to use this utility.
* Project-level documentation must be kept up to date (README.md)
* ALL code must be unit-tested (target coverage of 90%).  When you are generating code, you MUST have positive and negative test cases associated with the code.
* Avoid being overly verbose when writing documentation.  Include essential details.  Assume that a human has to digest all of this information, but also assume that an AI is using this to further develop the solution.
* All architectural and informational docs live in `docs/`.  Refer to those docs when making changes, as there is important information in there.
* When you are done creating code, ensure the application compiles and an updated executable binary exists in `dist/`.