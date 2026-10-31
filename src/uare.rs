//! Arbitrage UARE sans LLM (L1). La décision est explicite et auditée ; L2 l'automatisera.
use crate::dice::{self,DiceRng,RollMode,RollResult};
use anyhow::{bail,Result};
use serde::{Deserialize,Serialize};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Ability {Strength,Dexterity,Constitution,Intelligence,Wisdom,Charisma}
impl Ability {
 pub const ALL:[Self;6]=[Self::Strength,Self::Dexterity,Self::Constitution,Self::Intelligence,Self::Wisdom,Self::Charisma];
 pub fn label(self)->&'static str {match self {Self::Strength=>"Force",Self::Dexterity=>"Dextérité",Self::Constitution=>"Constitution",Self::Intelligence=>"Intelligence",Self::Wisdom=>"Sagesse",Self::Charisma=>"Charisme"}}
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Stats {pub strength:i32,pub dexterity:i32,pub constitution:i32,pub intelligence:i32,pub wisdom:i32,pub charisma:i32}
impl Default for Stats {fn default()->Self{Self{strength:10,dexterity:10,constitution:10,intelligence:10,wisdom:10,charisma:10}}}
impl Stats {
 pub fn value(&self,a:Ability)->i32{match a{Ability::Strength=>self.strength,Ability::Dexterity=>self.dexterity,Ability::Constitution=>self.constitution,Ability::Intelligence=>self.intelligence,Ability::Wisdom=>self.wisdom,Ability::Charisma=>self.charisma}}
 pub fn modifier(&self,a:Ability)->i32{(self.value(a)-10).div_euclid(2)}
 pub fn validate(&self)->Result<()>{for a in Ability::ALL{if !(1..=30).contains(&self.value(a)){bail!("Caractéristique hors limites");}}Ok(())}
}

pub fn proficiency_bonus(level:u8)->i32{2+((level.saturating_sub(1))/4) as i32}

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Decision { AutoSuccess, AutoFailure, AbilityCheck }
impl Decision {pub fn label(self)->&'static str{match self{Self::AutoSuccess=>"Réussite sans jet",Self::AutoFailure=>"Impossible / échec",Self::AbilityCheck=>"Test au d20"}}}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ActionRequest {
 pub key:String,pub campaign_id:String,pub actor_id:String,pub text:String,
 pub decision:Decision,pub ability:Ability,pub proficient:bool,pub dc:u8,
 pub roll_mode:RollMode,pub target_id:Option<String>,pub hp_delta:i32,
 pub success_text:String,pub failure_text:String,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ActionOutcome {pub success:bool,pub roll:Option<RollResult>,pub effective_dc:Option<u8>,
 pub narration:String,pub applied_hp_delta:i32,pub target_hp:Option<i32>,
 #[serde(default)] pub applied_effect:Option<String>}

pub fn resolve<R:DiceRng>(request:&ActionRequest,stats:&Stats,level:u8,rng:&mut R)->Result<ActionOutcome>{
 if request.text.trim().is_empty()||request.text.chars().count()>4000{bail!("Action vide ou trop longue");}
 if !(-9999..=9999).contains(&request.hp_delta){bail!("Variation de PV hors limites");}
 if request.target_id.is_none()&&request.hp_delta!=0{bail!("Choisir une cible pour modifier ses PV");}
 if request.decision==Decision::AbilityCheck && !(1..=30).contains(&request.dc){bail!("DD hors limites (1–30)");}
 let (success,roll,dc)=match request.decision{
  Decision::AutoSuccess=>(true,None,None),Decision::AutoFailure=>(false,None,None),
  Decision::AbilityCheck=>{
   let bonus=stats.modifier(request.ability)+if request.proficient{proficiency_bonus(level)}else{0};
   let rolled=dice::roll_with(&format!("1d20{:+}",bonus),request.roll_mode,rng)?;
   let success=rolled.total>=request.dc as i32;
   (success,Some(rolled),Some(request.dc))
  }
 };
 let template=if success{request.success_text.trim()}else{request.failure_text.trim()};
 let narration=if template.is_empty(){if success{"Action réussie."}else{"Action échouée."}}else{template};
 Ok(ActionOutcome{success,roll,effective_dc:dc,narration:narration.to_owned(),applied_hp_delta:0,target_hp:None,applied_effect:None})
}

#[cfg(test)] mod tests{
 use super::*;
 struct Fixed;impl DiceRng for Fixed {fn bounded(&mut self,_:u32)->Result<u32>{Ok(12)}}
 #[test]fn check_modifiers_and_success()->Result<()> {
  let request=ActionRequest {key:"r".into(),campaign_id:"c".into(),actor_id:"a".into(),text:"Escalade".into(),decision:Decision::AbilityCheck,ability:Ability::Strength,proficient:true,dc:16,roll_mode:RollMode::Normal,target_id:None,hp_delta:0,success_text:"Au sommet".into(),failure_text:"Chute".into()};
  let mut stats=Stats::default();stats.strength=16;
  let result=resolve(&request,&stats,1,&mut Fixed)?;
  assert_eq!(result.roll.unwrap().total,17);assert!(result.success);Ok(())
 }
}
