//! LOOP-TRAIN-1 offline training. The OFFLINE marker planner and native writer live in
//! [`offline`]; its native callers (statue USE, logout flush, admission settlement) are
//! slot work.

pub(crate) mod offline;
