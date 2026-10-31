//! Tirages uniformes non biaisés ; le moteur ne fournit jamais des jets inventés.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum RollMode { Normal, Advantage, Disadvantage }
impl RollMode { pub fn label(self)-> &'static str {match self {Self::Normal=>"Normal",Self::Advantage=>"Avantage",Self::Disadvantage=>"Désavantage"}} }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollResult { pub expression:String, pub faces:u32, pub rolls:Vec<u32>,
    pub retained:Vec<usize>, pub modifier:i32, pub total:i32, pub mode:RollMode }

pub trait DiceRng { fn bounded(&mut self, upper:u32) -> Result<u32>; }

pub struct SystemRng;
impl DiceRng for SystemRng {
    fn bounded(&mut self, upper:u32)->Result<u32> {
        if upper==0 {bail!("Dé à zéro face");}
        // Rejet des échantillons au-dessus du plus grand multiple de upper dans 2^32.
        let zone=(u32::MAX as u64)+1;
        let limit=zone-(zone % upper as u64);
        loop {
            let mut bytes=[0_u8;4];
            getrandom::fill(&mut bytes).map_err(|e|anyhow::anyhow!("Erreur du générateur aléatoire système : {e}"))?;
            let n=u32::from_le_bytes(bytes) as u64;
            if n<limit {return Ok((n % upper as u64) as u32 + 1);}
        }
    }
}

/// Dés simples : NdM, NdM+X, NdM-X, avec N <= 100 et 2 <= M <= 1000.
/// Avantage/désavantage seulement pour 1d20.
pub fn roll_with<R:DiceRng>(expr:&str, mode:RollMode, rng:&mut R)->Result<RollResult> {
    let normalized:String=expr.chars().filter(|c|!c.is_whitespace()).collect();
    let text=normalized.to_ascii_lowercase();
    let (n_text,tail)=text.split_once('d').ok_or_else(||anyhow::anyhow!("Format attendu : NdM+bonus"))?;
    let n:usize=if n_text.is_empty(){1}else{n_text.parse()?};
    if !(1..=100).contains(&n){bail!("Nombre de dés hors limites (1–100)");}
    let split=tail.find(|c|c=='+'||c=='-').unwrap_or(tail.len());
    let faces:u32=tail[..split].parse()?;
    if !(2..=1000).contains(&faces){bail!("Faces hors limites (2–1000)");}
    let modifier:i32=if split==tail.len() {0}else{tail[split..].parse()?};
    if !(-10000..=10000).contains(&modifier){bail!("Modificateur hors limites");}
    if mode!=RollMode::Normal && (n!=1||faces!=20){bail!("Avantage et désavantage réservés au d20");}
    let count=if mode==RollMode::Normal {n}else{2};
    let mut rolls=Vec::with_capacity(count);
    for _ in 0..count { rolls.push(rng.bounded(faces)?); }
    let retained=if mode==RollMode::Normal {(0..n).collect()}
        else if (mode==RollMode::Advantage && rolls[0]>=rolls[1]) ||
                (mode==RollMode::Disadvantage && rolls[0]<=rolls[1]) {vec![0]} else {vec![1]};
    let total=retained.iter().map(|&i|rolls[i] as i32).sum::<i32>()+modifier;
    Ok(RollResult{expression:normalized,faces,rolls,retained,modifier,total,mode})
}

pub fn roll(expr:&str,mode:RollMode)->Result<RollResult>{roll_with(expr,mode,&mut SystemRng)}

#[cfg(test)] mod tests {
    use super::*;
    struct Sequence(Vec<u32>,usize);
    impl DiceRng for Sequence {fn bounded(&mut self,upper:u32)->Result<u32>{let n=self.0[self.1];self.1+=1;assert!(n<=upper);Ok(n)}}
    #[test] fn advantage_and_disadvantage()->Result<()> {
        let a=roll_with("1d20+5",RollMode::Advantage,&mut Sequence(vec![4,17],0))?;
        assert_eq!(a.total,22); assert_eq!(a.retained,vec![1]);
        let d=roll_with("d20-2",RollMode::Disadvantage,&mut Sequence(vec![3,19],0))?;
        assert_eq!(d.total,1);assert_eq!(d.retained,vec![0]);Ok(())
    }
    #[test] fn ordinary_and_invalid()->Result<()> {
        let r=roll_with("2d6+3",RollMode::Normal,&mut Sequence(vec![4,5],0))?;
        assert_eq!(r.total,12);assert_eq!(r.rolls,vec![4,5]);
        assert!(roll("20d0",RollMode::Normal).is_err());
        assert!(roll("2d20",RollMode::Advantage).is_err());
        assert!(roll("1d6+10001",RollMode::Normal).is_err()); Ok(())
    }
}
