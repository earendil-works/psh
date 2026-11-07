//! Arithmetic over the shell's variables: psh-syntax's evaluator (`eval_arith`) with
//! the vproc as its environment. Arrays come later; `x[0]` is the scalar `x`.

use psh_syntax::{ArithEnv, ArithIndex, EvalError, eval_arith};

use crate::state::Vp;

struct Env<'a> {
    vp: &'a mut Vp,
}

impl ArithEnv for Env<'_> {
    type Error = String;

    fn is_assoc(&mut self, _name: &[u8]) -> bool {
        false
    }

    fn get(
        &mut self,
        name: &[u8],
        index: Option<ArithIndex<'_>>,
    ) -> Result<Option<Vec<u8>>, String> {
        if !matches!(index, None | Some(ArithIndex::Num(0))) {
            return Ok(None);
        }
        let name = String::from_utf8_lossy(name);
        Ok(self.vp.var(&name).map(<[u8]>::to_vec))
    }

    fn set(
        &mut self,
        name: &[u8],
        index: Option<ArithIndex<'_>>,
        value: i64,
    ) -> Result<(), String> {
        let name = String::from_utf8_lossy(name).into_owned();
        if !matches!(index, None | Some(ArithIndex::Num(0))) {
            return Err(format!("{name}: arrays are not supported yet"));
        }
        if self.vp.vars.get(&name).is_some_and(|v| v.readonly) {
            return Err(format!("{name}: readonly variable"));
        }
        self.vp.set_var(&name, value.to_string().into_bytes());
        Ok(())
    }
}

/// Evaluates `text`; the error is bash's message without the `line N:` prefix.
pub(crate) fn eval(vp: &mut Vp, text: &[u8]) -> Result<i64, String> {
    match eval_arith(text, &mut Env { vp }) {
        Ok(v) => Ok(v),
        Err(EvalError::Env(msg)) => Err(msg),
        Err(e) => Err(e.bash_text().unwrap_or_default()),
    }
}
