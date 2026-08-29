# Product surface

The thing you sell is **Øneiro**: a lucid-dreaming instrument (EEG or
watch REM detect, haptic cue, journal). The hosted front door is
https://oneiro-seven.vercel.app. ATRIUM does not sell.

## What is for sale

- **Øneiro** is the product. Private repo `xingxerx/oneiro` (engine,
  watch, haptic firmware) plus `xingxerx/oneiro_website` (the Vercel
  site). Cohort 03 waitlist is the current offer: wristband, listener,
  three nights of calibration, NDA.
- The waitlist CTA (`knock`) currently opens a mailto to
  `oneiro.orieno@gmail.com`. There is no server-side list and no
  payment. The "247 sleepers" line is copy, not a count.
- **VEYN** is the local sensor bus Øneiro can ride (ELv2). Intero is
  the first-party app on that bus. Keep them as infrastructure unless
  a customer is buying a daemon.
- LIMEN stays a library. DUCTEI and Qallow stay plumbing. ATRIUM stays
  the constitution.

## Live state (August 29 2026)

- Site is up: https://oneiro-seven.vercel.app
- Knock: mailto form, not a database.
- VEYN security PR 77 merged. VEYN GitHub blurb corrected to ELv2.
- Next charge step on this path: keep the mailto until you want a
  real list or a Stripe deposit. That work belongs in
  `oneiro_website`, not ATRIUM.

## Routing

Work that is "sell the lucid-dreaming cohort / site / wristband" goes
to `oneiro` and `oneiro_website`. Work that is "sensor bus / Intero"
goes to VEYN. Do not open commercial copy in DUCTEI, LIMEN, or Qallow.
