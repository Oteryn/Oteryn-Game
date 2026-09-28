# Tibia manual notes: accounts

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=accounts>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `7c1439345ebcadca115f910d957a9f582019d6a812ee719ffbd96c399822f74a`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Platform / web, not Game runtime; kept for completeness
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §accounts <heading>, capture 2026-09-28`.

## 7.1 Notes on Account Data

**Password** [platform]: Minimum 8 characters, letters+numbers required. Ideally uppercase/lowercase/numbers/symbols. Changes take effect immediately [platform].

**Email Address** [platform]: Account identity anchor; required for login and password recovery. One email per account; reassignable later [platform].

**Account Confirmation and Recovery Key** [platform]:
- Confirmation link via email; required to unlock account features [platform]
- Recovery key (20-digit) generated post-confirmation; essential for account recovery and two-factor setup [platform]
- Recovery key must be written down and re-entered to validate; becomes active only after verification [platform]
- Can be reset via "Recovery Setup" on account page [platform]

**Secondary Email Address** [platform]:
- Set after account confirmation and recovery key creation [platform]
- Must differ from primary and cannot use email-alias variations [platform]
- Alternative recovery route if primary email compromised [platform]

## 7.2.1 Account Status

Summary box showing Premium/free status with expiration date [platform]. Three action buttons: Manage Account, Get Premium (if non-Premium), Logout [platform]. Automatic session logout after 45 minutes inactivity [platform].

## 7.2.2 Download Client

Direct link to latest Tibia client download [platform]

## 7.2.3 Characters

Character list displays name, vocation, level, houses owned, guild memberships, and visibility status [platform]. Per-character actions: Edit, Delete/Undelete [platform].

**Delete Character** [platform]: Scheduled for deletion; 60-day reversal window. After 60 days, permanent deletion. Deleted characters don't count toward 25-character limit; deleted names reserved 6 months [platform].

**Undelete Character** [platform]: Reverse deletion within 60-day window [platform].

**Character Data, Deaths, Kills** [platform]: View character metadata, recent death log (30 days), and recent kill record (10 weeks) [platform].

**Edit Character Information** [platform]: Hide account visibility (also hides all other characters), set public comment (viewed on character page), set forum signature (max 4 lines, 200 characters) [platform].

**Display Achievements** [platform]: Select up to 5 earned achievements for character page display [platform].

**Create Character** [platform]: New characters up to account limit of 25 active [platform].

## 7.2.4 General Information

Displays email addresses, account creation date, Premium status, last login, Tibia Coins balance, loyalty points, loyalty title, and conduct level (if violations exist) [platform].

**Change Password** [platform]: Requires current and new password; instantaneous [platform]

**Manage Emails** [platform]: Three change methods:
- By recovery key (instant) [platform]
- By recovery TAN SMS (instant, 24h window) [platform]
- By 30-day waiting period (if no recovery key/phone) [platform]

Pending changes trigger reminder email; cancellable during waiting period [platform]. Primary change requires new login within 24h [platform].

**Swap Emails** [platform]: Exchange primary and secondary with 30-day delay [platform]

**Add/Remove Secondary Email** [platform]: Removal triggers 30-day delay before re-adding [platform]

**Terminate Account** [platform]: Schedules all characters for deletion; reversible via "Cancel Termination" for 2-3 months [platform]

**Security Wizard** [platform]: Account security audit and improvement guide [platform]

## 7.2.5 Tell-A-Friend

**Invite New Players** [platform]: Share referral link; max 25 emails per 24h, 14-day resubmit window. Invitee receives mount (first 3 characters, 7-day window); inviter gets Recruiter Outfit on invitee's store purchase; outfit addons via multiple recruits; loyalty point sharing (1 point per 6 earned by invitee) [platform].

**Bring Back a Buddy** [platform]: Auto-email inactive buddies (no login/activity past 12 months); max 20 invites per 30 days. Returning buddy receives welcome-back gift on first login. Inviter gets 5 Premium days (30-day purchase by buddy) or 10 days (90+ day purchase) if buddy buys within 90 days [platform].

## 7.2.6 Loyalty Highscore Character

Select which character displays in Loyalty Highscores [platform]. Default: highest-level non-hidden character. Hidden characters cannot be selected [platform].

## 7.2.7 Products Available

One-stop ordering for Premium Time, Tibia Coins, extra services, and game code redemption [platform]

## 7.2.8 Products Ready to Use

Displays unpaid or prepared extra services (e.g., pending character world transfers) [platform]. Actions: View, Terminate (unpaid), Edit, Purchase (unpaid) [platform]

## 7.2.9 Tibia Token Exchange

Export Tibia Coins to blockchain (BEP-20 Tibia Tokens on BNB Smart Chain) or import tokens back to coins [platform].

**Requirements** [platform]:
- Active digital wallet with BNB gas reserves [platform]
- Confirmed, registered, two-factor-protected account [platform]
- Non-orange/red/black conduct level [platform]
- Agreement to Tibia Token Service + Reown cookies [platform]

**Process** [platform]: Connect wallet via Reown third-party service, export minimum 100 coins (incurs CipSoft fees + gas), or import tokens (gas only) [platform]

**Transaction History** [platform]: Status tracking (Pending, Verifying, Completed, Expired) with blockchain reference (BscScan.com) [platform]

**Liability** [platform]: CipSoft offers no support, compensation, or insurance for wallet hacks or fraud [platform]

## 7.2.10 History

**Premium History** [platform]: List of purchased Premium Time, vouchers, refunds, chargebacks [platform]

**Payments History** [platform]: All payment attempts, bank transfer details [platform]

**Coins History** [platform]: Tibia/Tournament Coin transactions (purchases, earnings, gifts, market trades, free compensations) with +/- notation [platform]

**Extra Services History** [platform]: All ordered extra services, paid/voucher status [platform]

**Vouchers History** [platform]: All received vouchers [platform]

## 7.2.11 Rule Violation Reports

Displays user-submitted reports from past 30 days with status (pending/processed) [platform]. Maximum 20 pending reports; invalid reports may restrict future submissions [platform]

## 7.2.12 Rule Violation Record

Audit trail of confirmed rule violations: date, character, severity, reason, punishment [platform]. Entries never erased but conduct level decreases over time [platform]. Accessible via account page if violations exist [platform]

## 7.2.13 Registration

Stores registrant name, address, nationality, phone number [platform]. Editable post-registration via "Edit" button; 30-day waiting period before changes finalize [platform]. Phone verification required if changed [platform]

## 7.2.14 Authenticator

**Two-Factor Authenticator App (recommended)** [platform]:
- Requires recovery key to link [platform]
- Should use separate device from game client [platform]
- Status buttons: Request (unlinked), Confirm/Cancel/Re-request/Link (in-progress), Unlink (active) [platform]

**Two-Factor Email Code (fallback)** [platform]:
- Risk of email delays [platform]
- Status buttons: Request (unlinked), Enter Email Code (in-progress), Deactivate (active) [platform]
- TAN codes valid 24h, single-use only [platform]

## 7.2.15 Email Notifications

Opt-in/out for special-offer emails [platform]

## 7.2.16 Tickets

Customer support tickets displayed 180 days from last reply [platform]. Unread tickets marked with indicator [platform]

## 7.2.17 Play Session Agreement

Consent toggleable for CipSoft to save play sessions for game experience improvement and rule enforcement [platform]

## 7.2.18 Personalised Ads on External Platforms

Consent toggleable for CipSoft to use account data for targeted advertising [platform]

## 7.2.19 Trusted Devices

Devices registered via two-factor login (checkbox "Trusted Device") skip two-factor for 30 days [platform]. Removed on two-factor method change or password change [platform]

## 7.3 Ordering Premium Time, Tibia Coins, or Recovery Key

**Tibia Coins packages** [platform]: 250, 750, 1500, 3000, 4500 coins; transferable or initially non-transferable tiers [platform]

**Premium Time** [platform]: 30, 90, 180, 360 days [platform]

**Payment methods** [platform]: Vary by country; official resellers available. Prices vary per method and exchange rate; resellers set own prices [platform]

**Ordering flow** [platform]: (1) Select service/country/product, (2) Enter payment data per method, (3) Confirm order, (4) Finalize via summary. Bank transfer requires reason-code entry; credit card verification up to 3 days [platform].

**Recovery Key letter** [platform]: Sent to registration address, not payment address. Fee required unless mobile phone registered (free SMS via recovery TAN) [platform]

## 7.4 Loyalty System

**Loyalty Points** [platform]: 1 point per Premium day used (all sources: purchases, vouchers, gifts, compensation, rewards). Viewable in General Information [platform]

**Loyalty Highscores** [platform]: Top 300 accounts per world by loyalty points [platform]. Default character displayed is highest-level non-hidden; player-selectable [platform]

**Loyalty Titles** [platform]: Account-wide title based on loyalty points (Scout at 50, Sentinel at 100, ..., Enlightened at 7000+) [platform]. Visible on non-hidden characters [platform]

**Skill Bonus** [platform]: Per 360 loyalty points, 5% skill point bonus (max 50% at 2160+ points) [platform]

## 7.5 Account Registration

**Why Register** [platform]:
- Enhanced security [platform]
- Account recovery via postal mail if other methods fail (fee-based; free with registered phone) [platform]
- Recovery TAN via SMS (phone-registered only) [platform]

**How to Register** [platform]: Click "Register Account", enter name/address/optional phone, confirm via email link and 30-day waiting period [platform]

**Modify Registration** [platform]: Click "Edit", change data, confirm via email link. Changes finalize after 30-day waiting period and final confirmation [platform]. New phone numbers require verification [platform]

---

## Open questions for Oteryn

- Will Oteryn implement loyalty points and premium-time-based skill bonuses?
- Should Oteryn support blockchain token exchange, or limit to in-game trading?
- How should character deletion/recovery be handled in Oteryn's architecture (database soft-delete vs. hard-delete)?
- Will Oteryn enforce 25-character account limit, or adjust based on server resources?
- Should trusted devices bypass two-factor indefinitely, or enforce periodic re-auth?
