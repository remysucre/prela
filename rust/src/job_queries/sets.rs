// Named tuple-constants from queries.jl. Each call constructs a fresh Vec so
// the owning Filter holds its own InVec — the engine consumes by value.

pub fn kw7()  -> Vec<&'static str> { vec!["murder","violence","blood","gore","death","female-nudity","hospital"] }
pub fn kw8()  -> Vec<&'static str> { vec!["superhero","sequel","second-part","marvel-comics","based-on-comic","tv-special","fight","violence"] }
pub fn kw10() -> Vec<&'static str> { vec!["superhero","marvel-comics","based-on-comic","tv-special","fight","violence","magnet","web","claw","laser"] }

pub fn voice3() -> Vec<&'static str> { vec!["(voice)","(voice) (uncredited)","(voice: English version)"] }
pub fn voice4() -> Vec<&'static str> { vec!["(voice)","(voice: Japanese version)","(voice) (uncredited)","(voice: English version)"] }

pub fn writer5() -> Vec<&'static str> { vec!["(writer)","(head writer)","(written by)","(story)","(story editor)"] }
pub fn genre6()  -> Vec<&'static str> { vec!["Horror","Action","Sci-Fi","Thriller","Crime","War"] }
pub fn murder4() -> Vec<&'static str> { vec!["murder","murder-in-title","blood","violence"] }

pub fn nordic8()  -> Vec<&'static str> { vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German"] }
pub fn nordic9()  -> Vec<&'static str> { vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","English"] }
pub fn nordic10() -> Vec<&'static str> { vec!["Sweden","Norway","Germany","Denmark","Swedish","Danish","Norwegian","German","USA","American"] }

pub fn link3() -> Vec<&'static str> { vec!["sequel","follows","followed by"] }
