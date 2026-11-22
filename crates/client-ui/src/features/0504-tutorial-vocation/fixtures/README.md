These are real command-result payloads captured on 2026-11-15 by
`oteryn-gameplay-dispatch --lib tutorial_vocation_offer_and_submit_real_dispatch`.
The test admits the native Character, enters the accepted Newhaven tutorial Island
state, observes the five-card offer, submits Paladin through command 6243, and checks
one build receipt and the profession kit. Set `OTERYN_TUTORIAL_VOCATION_CAPTURE`
to a directory to regenerate both payloads. Client reducer and Bevy tests replay
these exact bytes; their default paths do not skip when capture variables are absent.
