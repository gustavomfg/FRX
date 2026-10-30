"""Clearly synthetic TRAINING ONLY examples; never included in evaluation."""
import random

def generate():
    rng=random.Random(81);rows=[]
    outcomes={
      1:['fixed the crashes','resolved my launch failure','made the game run correctly','eliminated the stuttering','solved the problem for me','got rid of the graphical glitches','improved performance','made the game stable','let me play without crashing','it works now','the crash disappeared'],
      2:['did not fix the crashes','did not help','made no difference','had no effect on the problem','did not resolve it','the game still crashes','the issue persists','it still freezes at startup','gave me the same error','it does not work for me'],
      3:['caused distorted geometry and crashes','made performance worse','introduced severe stuttering','broke the graphics','caused a black screen','made it crash on startup','resulted in new visual artifacts','made the game slower'],
      4:['fixed the crash but caused graphical glitches','improved FPS but it still crashes','helped partially but not completely','let me launch but the textures are corrupted','reduced stutter but introduced freezes'],
    }
    actions=[('enable','enabled X_FLAG=1'),('disable','disabled X_FLAG'),('install','installed the required library'),('remove','removed the extra library'),('update','updated the driver'),('replace','replaced the configuration file'),('switch','switched to another Proton version'),('configure','changed the graphics setting')]
    for state,action in actions:
      for gold,phrases in outcomes.items():
       for phrase in phrases:
        for form in [f'I {action} and {phrase}.',f'After I {action}, {phrase}.']:
         target='X_FLAG' if state in ('enable','disable') else '<procedure>'
         # Use recognized names but normalize to TARGET in the feature pipeline.
         form=form.replace('X_FLAG','PROTON_SYNTHETIC_FLAG');target=target.replace('X_FLAG','PROTON_SYNTHETIC_FLAG')
         rows.append(dict(text=form,target=target,gold=gold,gold_state=state,origin='synthetic-training-template',id='syn-'+str(len(rows))))
      for phrase in ['Try '+action+' to fix the crashes.','You could '+action+' if the game crashes.','Has anyone '+action+'?','I will try '+action+' later.']:
       rows.append(dict(text=phrase.replace('X_FLAG','PROTON_SYNTHETIC_FLAG'),target='PROTON_SYNTHETIC_FLAG' if state in ('enable','disable') else '<procedure>',gold=0,gold_state='unknown',origin='synthetic-training-template',id='syn-'+str(len(rows))))
    # Decouple diagnostic flags/quoted successes from actual results.
    for text in ['I am running with PROTON_LOG=1. The game crashes.','> PROTON_SYNTHETIC_FLAG=1 fixed it for me.\nI have not tried this.','I upgraded the driver and it works. My command is PROTON_SYNTHETIC_FLAG=1 %command%.']:
      rows.append(dict(text=text,target='PROTON_LOG' if 'PROTON_LOG' in text else 'PROTON_SYNTHETIC_FLAG',gold=0,gold_state='unknown',origin='synthetic-training-template',id='syn-'+str(len(rows))))
    rng.shuffle(rows);return rows
