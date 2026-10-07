# Jarcade

A small Rust / Macroquad arcade with shared game and UI code for desktop, web,
Android, and iOS. Play Snake, Minesweeper, Fih, Coupe, Dicksit, Wavelength, Table tennis, Wolvesville, Codenames, Nonograms, and Sudoku.

Public site: <https://jarcade.jaypussy.site>. Source: <https://github.com/j0taaa/jarcade>. Hosting uses this PC and the
existing Cloudflare/Tailscale route. This PC must stay awake and both it and
`cloud` must remain online.

## Run

```sh
cargo run --release
```

Web (Rust, Python 3; no Node dependency for building):

```sh
rustup target add wasm32-unknown-unknown
bash scripts/build-web.sh
cargo run --release --features server --bin jarcade-server
```

Open <http://localhost:8091>. A plain static server can run solo games and local Wavelength; online Coupe, Dicksit, Wolvesville, and Codenames require the room service. To update the public site:

```sh
host-app proxy jarcade 8091
```

The web build versions asset URLs by content hash so a new release does not
reuse an older WebAssembly binary or JavaScript adapter.

## Design and controls

- White by default, with Inter typography and a green accent. Game cards show
  a high-resolution gameplay image produced by the actual board renderer.
- High-DPI rendering uses the display's native pixel ratio. UI geometry stays
  in logical points, including touch coordinates and swipe thresholds.
- The launch gallery uses two columns on phones and more on larger screens.
  Swipe, scroll, or use Page Up/Down when the gallery exceeds the screen;
  card taps activate on release, and keyboard focus scrolls cards into view.
- Arrows / WASD steer; swipes and directional buttons work on touch screens.
- Space / P starts, pauses, or resumes. Enter starts or retries.
- Escape pauses; press again to go home. Tab / Shift+Tab and Enter navigate UI.
- Score and preferences persist in browser storage or the OS app-data folder.
  Storage failures do not prevent play.

## Power saver

Power saver is **off by default**. Turning it on switches the app and browser
surround to true black, removes the checkerboard and smooth movement, and
renders on 140 ms game ticks and input events. The board boundary and game
preview remain visible. Resolution stays sharp in both modes.

Normal mode slides the head and tail along a continuous trail, keeps interior
bends in place, eases the head rotation through turns, and renders at the display/browser refresh rate without an application FPS cap. Game
speed is independent of drawing frequency (tested through 240 Hz). Starting
begins moving immediately; pausing/resuming preserves the animation phase.
Turns apply at the next cell boundary, with two quick turns buffered in event
order. Keyboard, touch buttons, and short 8-point swipes are handled before
the game tick; even complete gestures between frames are preserved.
Actual FPS
depends on the browser, operating system, and hardware. Both modes use an event-driven loop:
menus and paused/finished rounds have no recurring application timer. Active Wolvesville phases redraw their countdown once a second; the server sleeps until the next phase deadline. There is
no audio, network polling, or multisampling. Multiplayer uses event-driven WebSockets, with control-frame heartbeats once a minute that do not redraw the interface.

Background/focus changes and long timing gaps pause play rather than advancing
through unseen moves. Actual battery savings depend on the device and backend;
physical-device profiling is still needed.

Old v1 saves migrate once to the new white/default-off preferences, preserving
best scores. The v3 format remembers power saver, haptics, and the optional FPS counter.
Existing v2 preferences migrate with the counter off.

## FPS counter

Enable **Settings → FPS counter** to show a small bottom-right readout. It measures
actual rendered frames over half-second windows, including power saver’s slower
cadence. Menus, paused Snake, and Minesweeper show `FPS · idle`: the counter never
creates a rendering loop for an otherwise still screen. The choice is saved.

## Minesweeper

Choose a board before starting: Small (9×9, 10 mines), Medium (16×16, 40 mines),
or Large (30×16, 99 mines). The first reveal and neighboring tiles are safe.
Empty areas open automatically. Reveal every safe tile to win; flags are optional.

- Tiles start at 48–78 logical points wide, using more of the available height on small boards. Pinch with two fingers to zoom, or use
  **− / Fit / +** above the board. Fit shows the whole board. Keyboard **− / +**
  zoom and **0** fits; zoom ranges from 20% to 250%, without changing the native
  display resolution. Pinches never reveal or flag tiles, even when one finger lifts.
- The board uses nearly the full screen width with compact header and controls.
  Drag/swipe the board in either
  direction; mouse wheels and trackpads scroll, and Shift+wheel moves horizontally.
  Scroll thumbs show your position. Dragging never reveals or flags a tile.
- The fixed Reveal/Flag selector has icons and a solid active segment: green for
  Reveal, coral for Flag. In power saver, the active segment uses a bright outline.
- Tap/click (release) to use the selected tool; right-click always toggles a flag.
- Arrow keys select and automatically scroll to a tile; Enter/Space uses the
  selected tool, and F toggles a flag.
- Tap a revealed number to open neighbors when its flag count matches.
  Incorrect flags can uncover a mine.
- The top-right settings button returns to the size picker. New board after a
  completed round also opens the picker, remembering the current choice.
- Minesweeper redraws only on interaction (and during mouse dragging), in both
  normal and power saver modes. The launch image uses the same board renderer.

## Fih

Fih fills the viewport, with its controls drawn inside five illustrated rooms.
Tap the room title to open the room picker, use the arrows, or tap a stat icon:

- **Kitchen**: 12 foods across Fresh, Meals, Treats and Drinks. Choose stock in
  the pantry or buy food in the shop; drag it into Fih’s mouth to feed.
  Fih follows the food with its eyes, opens its mouth, and chews. Taps, drops
  elsewhere, canceled gestures, and multiple fingers never consume stock.
  Flakes are always free and unlimited. Foods have different nutrition, joy,
  and health benefits. Failed feeding never consumes stock.
- **Bathroom**: drag the soap and rub across both sides of Fih to build lather,
  then use the shower to rinse. Holding still or tapping cannot wash Fih.
- **Bedroom**: switch the light off to sleep, or on to wake. Sleeping restores
  energy, including offline. The wardrobe is accessible only here.
- **Playroom**: throw a ball that bounces inside the room; Fih follows it and gets happy.
  Eight mini-games are exclusive to Fih.
- **Clinic**: a separate potion shop sells Health (8 coins), Energy (6), and Recovery
  (16). Purchased potions stay in the cabinet; drag them into Fih’s mouth to use them.

The fish is an original front-facing character rendered entirely from Rust
curves and gradient meshes, including its fins, tail, eyes, belly and mouth.
The reference image is not bundled or rendered. Fin flaps, tail movement,
bobbing, blinking, sleeping and mouth expressions can animate independently.
The same character renderer is used in rooms, clothing previews, games and the
arcade card. Rooms are hand-authored SVG rectangles, ellipses and polygons, drawn as
vectors at the current display resolution, with no raster backgrounds or
generated images. Artwork editing is documented in
[assets/fih/README.md](assets/fih/README.md).

The bedroom shirt icon opens the wardrobe: eight free body colors, 12 clothing choices,
12 hats (including the bare/no-hat defaults), and eight room color themes.
Clothes follow the same curved body silhouette in every view and sit beneath
the face. Every item has a visual preview; browsing never spends coins. Use **Buy & equip**
or **Equip** to confirm a selection. Owned items never charge again.

Mini-games:

- **Pearl Catch**: 30 seconds; drag or use Left/Right (A/D) to catch pearls and
  dodge urchins. Consecutive catches build scoring streaks; missing a pearl
  or hitting a hazard resets the streak. Falling objects speed up over time.
- **Bubble Pop**: 20 seconds; pop rising pearl bubbles before they expire,
  avoid spiky hazards, and build streaks. On keyboard, arrows aim the cursor
  and Space pops at that position. Space never automatically targets a bubble.
- **Memory Reef**: complete three rounds of six matching pairs within 60 seconds.
  Each round gives less time to memorize the cards. Mismatches turn back over after
  a short delay; matched pairs stay revealed. Card flips animate.
- **Reef Hop**: 30 seconds; tap the side with the nearest stepping stone, or
  use Left/Right. Jumps follow a smooth arc. Wrong choices and waiting until
  a stone sinks cost a heart. The deadline gets shorter as your score grows.
  Rapid double taps are blocked.
- **Reef Dash**: tap or Space to swim up through gaps in kelp for up to 45
  seconds. Obstacles get faster and more frequent. Collisions cost hearts;
  each obstacle can charge only once, with a brief recovery period after hits.
- **Shell Breaker**: drag or use Left/Right to move the paddle, bounce a pearl,
  and clear shells. Each new wave moves faster and narrows the paddle.
- **Pearl Slalom**: steer around falling urchins and collect pearls. Falling speed
  and spawn frequency increase as levels advance.
- **Tide Beats**: tap one of three lanes (or A/S/D) when a bubble crosses the
  target line. Perfect hits and streaks score more; the rhythm speeds up.

All mini-games increase their difficulty during play.
Completed rounds award 5 coins plus 2 per point, happiness and XP, and save a
best score. Rewards are paid once. Leaving an unfinished round pays nothing.
For keyboard care, Tab to food or soap and Enter to pick it up; arrow presses
move it, then Enter releases it. Soap needs movement over the body, just as
on touch. Pause and focus/background interruptions stop mini-games. Escape closes a
menu or mini-game before leaving Fih. Tab and Enter operate icons and cards.

Food, joy, cleanliness, energy, and health change with elapsed real time.
Neglected pets can recover through free basic food and washing. Affection has
a 30-second cooldown. A level is earned every 100 XP. Progress is local to each
browser/device and saved separately from app settings in `jarcade.fih.v1` or
native `fih.txt`. The v3 data format preserves existing v1/v2 stats, coins, purchases,
selections, scores and pantry inventory, adding three scores and potion stock. Malformed
saves and clock changes are handled safely. Native writes replace the file
atomically; save failures are shown without blocking play.

Holding a finger on the room makes Fih follow it with its eyes. Hunger, tiredness,
illness and low happiness change its face; dirt appears on its body. Full fish
refuse food with a head shake and “NAH”, preserving food stock. Soap bubbles
follow actual rubbing positions. Coin and level indicators share a centered line.
App settings are available from the arcade launch screen, never inside Fih.

Visible rooms breathe, bob, flap fins and blink in normal mode. Care actions
animate feeding, washing, affection, healing, clothing changes and sleep/wake.
Menus stop animating after a brief clothing animation; off-screen pets do no
background work. Active rooms and mini-games follow the display refresh rate
in normal mode. Stats refresh once per minute. In power saver idle animation
stops; interaction and care animations render only while needed. Power saver keeps the screen black, hides room artwork,
and limits active rendering to 30 FPS while preserving resolution and rules.

## Haptics

Haptics are enabled by default and can be turned off in Settings. Short feedback
is triggered for UI actions, accepted turns, eating, and round outcomes. Repeated
taps are rate limited; vibration is never emitted continuously with movement.

- Web uses the [Vibration API](https://developer.mozilla.org/en-US/docs/Web/API/Vibration_API)
  where available. Unsupported browsers show a disabled control. iPhone Safari
  does not expose this API; a website cannot promise vibration there. Browser
  capability detection cannot guarantee a device actually contains a motor.
- Android uses `Vibrator` / `VibrationEffect` with the normal `VIBRATE` permission.
- iOS uses UIKit impact and notification feedback generators on the main queue.
- Desktop without a supported vibration API safely does nothing.

Physical haptic feel and mobile packaging have not been verified on devices.

## Mobile builds

Install the Android SDK/NDK and Macroquad's `cargo-quad-apk`, then run
`cargo quad-apk build --release`. Package metadata is in `Cargo.toml`.
See the [upstream Android guide](https://macroquad.rs/articles/android/).

On macOS with Xcode, `bash scripts/build-ios.sh` creates an unsigned
`dist/Jarcade.app` for iPhone. For Apple Silicon simulators, use
`bash scripts/build-ios.sh aarch64-apple-ios-sim`, followed by
`xcrun simctl install booted dist/Jarcade.app` and
`xcrun simctl launch booted com.jarcade.arcade`. Real devices require signing and
provisioning; see the [upstream iOS guide](https://macroquad.rs/articles/ios/).

Linux and web release builds are verified locally. Android and iOS CI checks use their native SDK toolchains. Local multiplayer cross-checks require the Android NDK and Apple SDK; native mobile linking, signing, safe areas, and haptic hardware
still need SDK/device verification. Windows/macOS runtime testing remains.

## Checks and structure

```sh
cargo fmt --check
cargo clippy --locked --features server --all-targets -- -D warnings
cargo test --locked --features server
node --test scripts/*.test.cjs
bash scripts/build-web.sh
```

Rust tests cover Snake, Minesweeper, Fih, Table tennis, Wavelength, Nonograms, Sudoku, and online game rules and edge cases, first-move safety,
flood fill, chording, deterministic placement, FPS measurement,
frame-independent timing, input event ordering/modifiers, Wavelength handoff privacy, saves and dial projection, fast swipes/taps, continuous
Snake trails and head rotation, offline pet care, purchases, save validation,
mouth-only feeding, canceled drags, soap rubbing and body coverage, vector
asset validation, mini-game hazards/streaks/deadlines and single rewards, playroom ball physics and throws, potion inventory, preference migration, haptic rate limiting, phone
layout bounds, and high-DPI input mapping. Node's built-in test runner checks
web haptic patterns, unsupported/blocked APIs, theme synchronization,
accessibility status, unavailable browser storage, and animation-frame scheduling
under rapid input. No npm packages are used for building or these unit tests.

Optional browser checks (with the web server running on port 8080):

```sh
npm install --prefix /tmp/jarcade-browser-qa playwright
/tmp/jarcade-browser-qa/node_modules/.bin/playwright install chromium
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-fih.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-fih-layouts.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-fih-keyboard.cjs
# With the room service on port 8091:
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-multiplayer.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-multiplayer-layouts.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-wolves.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-codenames.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-nonograms.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-nonograms-endless.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-nonograms-ui.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-nonograms-marks.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-sudoku.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-home.cjs
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-card-table.cjs
# Table tennis image checks also require Python with Pillow and numpy:
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-table-tennis.cjs
```

Use `JARCADE_QA_URL` for another server and `JARCADE_CHROME` for an installed
Chrome executable. Checks use isolated browser storage and save screenshots
in `/tmp`. They cover phone care gestures, purchases, high-DPI rendering, idle
animation, all mini-games, compact layouts, and power-saver frame scheduling.
GitHub Actions runs formatting, linting, unit tests and the web build.

`src/fih.rs`, `src/fih_games.rs`, and `src/fih_extras.rs` contain pet and mini-game rules;
`src/fih_ball.rs` contains the playroom ball physics;
`src/fih_view.rs` renders its rooms, wardrobe, games, and launch preview;
`src/fih_character.rs` draws the animated character and `src/fih_art.rs` shares
vector rooms, clothing and icons. Pure care gesture and pose rules live in
`src/fih_interaction.rs`; `src/fih_svg.rs` parses the hand-authored room assets.
`src/snake.rs` and `src/minesweeper.rs` contain pure rules; `src/game_view.rs` renders both live gameplay
and the preview texture. `src/mines_view.rs` provides the equivalent Minesweeper
board and preview. `src/board_pan.rs` tests pan bounds, drag-versus-tap behavior, and keyboard scrolling.
`src/layout.rs`, `src/settings.rs`, and
`src/feedback.rs` hold testable policies. `src/main.rs` and `src/ui.rs` provide the
shared screens. Platform glue lives in `src/platform.rs` and `web/platform.js`.

Nonograms rules and original puzzle catalog live in `src/nonograms.rs`, with
the board, touch painting, and zoom controls in `src/nonograms_view.rs`.
Codenames rules and private projections live in `src/multiplayer/codenames.rs`;
`src/codenames_view.rs` draws its shared vector board and team setup.

Bundled Inter fonts and the Macroquad loader have their licenses alongside them.
See `assets/README.md` and `web/vendor/README.md` for provenance.

Game cards use two columns on portrait phones, three on tablets, and four on
wide desktops. Short landscape screens use four columns for the solo collection.
The iOS bundle opts into [ProMotion](https://developer.apple.com/documentation/bundleresources/information-property-list/cadisableminimumframedurationonphone);
the Metal view requests the attached screen’s maximum refresh rate. Native mobile
refresh behavior still requires physical-device verification.


## Wavelength

Choose **Multiplayer → Wavelength** for two or more people sharing one device.
It follows the unscored [local reference](https://wavelength-local.gabrieljotalizardo.chatgpt.site/):
one clue giver privately reveals the target between opposite ideas, gives a
spoken clue, then taps **Hide & pass**. The next player taps **Ready to guess**,
drags the needle, and confirms to reveal the colored proximity bands. Swap
roles for the next round. There are no room codes, accounts, scores, or timers.

Left/Right (or A/D) moves the needle; Shift makes larger steps. Space advances
phases; Tab and Enter navigate controls. The deck icon offers 60 prompt pairs
across Everyday, Playful, and Portuguese decks, plus editable custom extremes.
Tap either label on the dial to edit that side directly. Enter or tapping away saves;
Escape cancels, and blank edits keep the original. Editing preserves the target,
guess, and round, including during play; the edited pair stays saved for later rounds.
Labels support up to 120 characters; long titles shorten visually and remain complete
in the editor. Fields stay aligned when the screen or phone keyboard resizes.
Wavelength uses the reference’s warm cream background, including the browser surround,
and true black in power saver. Other games keep their white default.
Shuffle chooses another pair before a round. The same vector dial appears on
the launch card and renders sharply at native display resolution.

Targets are concealed on handoff, leaving, focus loss, and reload. A restored
guessing round returns to the handoff before continuing. Progress and custom
labels save separately in `jarcade.wavelength.v1` (web) or `wavelength.json`
(native). Storage failure does not block play. This is a casual same-device
privacy boundary; local saves contain the target. No network connection is
opened by this game. It plays offline once loaded, and idle screens have no
recurring application timer, including with the FPS counter enabled. Only a
held drag requests frames, at display refresh rate normally or 30 FPS in power
saver. The interface adapts to portrait and landscape layouts.

Pure rules and persistence validation are in `src/wavelength.rs`; the shared
view and gameplay preview are in `src/wavelength_view.rs`. Browser QA:

```sh
NODE_PATH=/tmp/jarcade-browser-qa/node_modules node scripts/qa-wavelength.cjs
```

## Multiplayer

For **Coupe** and **Dicksit**, open the Multiplayer category, enter a name, then create a room or join a
six-character room code. Everyone marks Ready; the host starts. Games run on
separate devices with private hands, secret submissions/votes, and explicit
challenge/block windows. Back returns to the arcade and preserves the seat;
Resume/Reconnect restores it, including after a refresh or server restart.
Leave explicitly removes a lobby seat, forfeits Coupe, or ends a Dicksit match.
The host can open a rematch. Offline players retain their seats until they
reconnect or leave; turn decisions have no automatic timeout.

Coupe uses a woodland table, coin chips, secret influence cards and role-colored
action tiles. On wide displays the hand and actions sit side by side. Dicksit
uses a lilac picture gallery with framed art, numbered cards, selection seals
and a clue/round ribbon. Room codes appear as shareable tickets; waiting rooms
use player tokens and open seats. Both keep a white canvas, true-black power
saver, touch and keyboard controls, and event-driven rendering when idle.

**Coupe** (2–6 players) uses Coup's base seven actions, role powers, challenges,
blocks, exchanges, forced coups at ten coins, and influence elimination. Its
original woodland portraits are drawn as Rust vectors. Regent = Duke,
Shade = Assassin, Corsair = Captain, Envoy = Ambassador, Sentinel = Contessa.
Each role has three cards. Everyone starts with two influences and two coins;
the first player starts with one coin at two players. Expansion factions,
Inquisitor, and optional advanced duel setup are outside the base game.

**Dicksit** (3–8 players) uses Dixit's storyteller, clues, secret decoys,
shuffled gallery, private votes, exact base scoring, discard recycling, and
30-point end. Normally players hold six cards; at three players they hold seven
and submit two decoys. There are **84 original AI-generated illustrations**.
Artwork and full prompts: [assets/reverie/README.md](assets/reverie/README.md).

Tap a clue to read it in full. On short wide displays, scores and clue move beside the picture gallery. Tap a picture for a large preview; Select then confirm with the fixed bottom
button. Browse by swipe, wheel, arrows or Page Up/Down. Drags and multi-touch
never select cards. Coupe's table scrolls on small displays. Tab/Shift+Tab and
Enter navigate controls. Web uses the phone/desktop text keyboard; Native Android and iOS provide an in-game touch keyboard for the shared text controls. The ? button explains rules and original role names.

These are independent adaptations with original names, interface, wording and
artwork; neither publisher's logos or card images are bundled. Rule references:
[Coup publisher](https://indieboardsandcards.com/our-games/coup/),
[Coup rules transcription](https://artofthegame.github.io/coup/rulebook.pdf),
[official Dixit 2021 rules](https://cdn.svc.asmodee.net/production-libellud/uploads/2022/03/DIXIT_REFRESH_RULES_US-UK-AU_BD.pdf).

**Wolvesville** (6–16 players) is a private-room social deduction adaptation with original vector portraits. Open `?game=wolves`, create a room, and invite friends on their own devices. The host can choose **Classic**, **Advanced**, or **Custom** in **Roles** before starting. Presets adapt to the player count; Custom has a role-count editor and ability previews. Setups need one role per seat, at least one wolf, more non-wolves than wolves, and no duplicated special roles. Changing a setup resets guests’ readiness.

Available roles: Villager, Werewolf, Seer, Doctor, Bodyguard, Gunner, Fool, Wolf seer, Serial killer, Aura seer, Medium, Witch, Avenger, Alpha werewolf, Junior werewolf, and Tough guy. Advanced presets introduce weighted pack votes, revenge marks, potions, revival, and delayed injury. The in-game **?** and **My role** panels explain each ability, remaining charges, private findings, and the public role pool. Rule reference: [official Wolvesville role descriptions](https://www.wolvesville.com/en/).

Night → Dawn → Discussion → Vote use 45/12/75/35-second deadlines, or advance early when every living player confirms. Select portraits, then explicitly lock your ability and separate pack hunt/poison choice. Wolves’ hunt votes use a unique plurality; the Alpha counts twice. Day executions require a strict majority of living players. Current votes remain hidden until resolved. The village wins after wolves and the serial killer are gone; wolves win at parity unless the killer remains; the Fool wins if voted out; the serial killer must be the final survivor. Forty uneventful days end in a draw.

Village, pack, and ghost channels enforce speaking and reading permissions on the server. The Medium’s night messages to ghosts are anonymous. Roles, findings, powers, and pack choices are projected per seat; death and the end of the match reveal roles. Back/disconnect preserves a seat and locked actions; explicit Leave forfeits. Timers continue without connected clients and survive server restarts, advancing only one overdue phase on recovery. The client updates the countdown at 1 Hz without a render loop on lobbies or finished matches. This version focuses on friends’ rooms and the listed roles, rather than public matchmaking, ranked modes, or the official app’s account features.

### Room service

```sh
cargo build --locked --release --features server --bin jarcade-server
JARCADE_STATIC_DIR=/absolute/path/to/dist \
JARCADE_DATA_DIR=/private/persistent/directory \
JARCADE_BIND=127.0.0.1:8091 target/release/jarcade-server
```

`/ws` serves the WebSocket protocol, `/health` reports service health, and other
paths serve the web build. Production uses the persistent [systemd service template](deploy/jarcade-server.service) and the existing HTTPS reverse proxy. Set its static directory to the published build snapshot; the process and private saves live outside the public folder. Native clients default to
`wss://jarcade.jaypussy.site/ws`; `JARCADE_SERVER_URL` overrides it for testing.
Server dependencies are gated by `--features server`, outside the mobile/web
app. Android needs INTERNET permission (declared) and the NDK for Rustls/ring.

The server validates membership, phase, legal actions, and card ownership;
clients receive only their own hidden cards and permitted roles, findings, and chat channels. Votes and decoy owners remain
secret until scoring. Each device stores a private bearer reconnect token;
invite URLs contain only the public game/room. Tokens and full shuffled game
state are saved atomically in `rooms.json` with mode 0600, outside static files.
The service prunes rooms inactive for 24 hours, caps rooms, messages and frames,
and checks browser WebSocket origins. Local saves and host backups contain
private seats/decks/roles and must remain private. The room host supplies availability;
this is a friends' room service, with no public matchmaking or accounts.

Tests cover challenges, double influence loss, blocks, forced coups, card
conservation, complete matches, all/mixed/no-correct scoring, the three-player
variant, deck recycling, private projections, simultaneous actions, revoked
connections, stale phases, and persistence/reconnect. Browser QA additionally
plays full matches through separate sessions and verifies idle rendering.

## Codenames

Choose **Multiplayer → Codenames**, or open `?game=codenames`. Invite 4–16 players
with a room code. Each player chooses Red or Blue and Operative or Spymaster;
each team needs exactly one spymaster and at least one operative. The host
chooses an original English or Portuguese word deck. Setup changes reset guest
readiness; everyone readies before the host starts.

The 5×5 board has nine agents for the starting team, eight for the other,
seven bystanders, and one assassin. Only spymasters receive the unrevealed key.
Give a one-word clue and a number; operatives select a word, then confirm its
reveal. A numbered clue permits up to the number plus one guesses. Zero and
unlimited permit any number of guesses. Make at least one guess before ending
the turn. A bystander or opposing agent ends the turn; finding a team's last
agent wins for that team, and revealing the assassin loses immediately.

The server checks roles, turns, clue format, board-word conflicts, and guesses.
Players discuss verbally; the game provides the shared word board and clue.
Disconnecting keeps the seat and private role; explicit Leave forfeits the
mission. The host can open a new mission after a result. Both themes render
only on events or active scrolling. Rules reference:
[official CGE rules](https://czechgames.com/files/rules/codenames-rules-en.pdf).
Artwork and word decks are original; this is an independent adaptation.

## Nonograms

Choose **Single player → Nonograms**, or open `?game=nonograms`. Choose a 5×5,
10×10, or 15×15 board. **Endless** generates fresh abstract mosaics offline;
**Pictures** keeps the twelve original picture puzzles. Every generated board
is checked for a unique solution obtainable through line deductions, without
guessing. Generation uses a bounded attempt budget with a guaranteed logical
fallback, and avoids repeating the last 32 puzzles at each size.
Numbers beside a row or column describe runs of filled cells, with at least
one empty cell between runs. Fill the picture to win; crossing empty cells is
optional, and extra filled cells prevent completion.

The selection screen uses pixel-art cards and saved-progress previews. In play,
the square, cross, and arrow icons select **Fill**, **Cross**, and **Move**; the
active tool stays labeled. The curved arrow undoes, the bulb gives a hint, and
the circular arrow opens reset. Minus/plus zoom; the corner icon fits the board.
Keyboard focus shows labels, and **?** opens a visual icon guide.

Use **Fill** or **Cross** to paint cells with a finger or mouse. Each stroke is
one undo operation; revisiting a cell does not toggle it repeatedly. A marked
cell must be cleared before placing the opposite mark; drags and hints respect
this rule too. Use **Move** to pan, or pinch/zoom to adjust the board. Canceled
or multitouch paint
gestures roll back their unfinished stroke. Undo, hints, and reset support
experimentation. Board progress is saved separately for every puzzle, in
browser storage or the native app-data folder. Endless keeps the current puzzle,
number, and progress separately for each size; finish it and choose **Next puzzle**
to generate another. Existing picture saves remain compatible. Invalid or
unavailable storage never prevents play. The game works offline after loading and has no recurring
idle timer, including with the FPS counter enabled.

Keyboard: arrows select cells, **Space** switches Fill/Cross, **Enter** paints
the selected cell, **X** uses Cross, and **F/C/V** choose
Fill/Cross/Move, **U** or **Ctrl+Z** undoes, **H** hints, and **R** opens reset.
Use **− / +** to zoom and **0** to fit the board.

## Table tennis

Choose **Single player → Table tennis**, or open `?game=table-tennis`. Play an offline match against the computer on Easy, Normal, or Hard. Move the racket with your mouse or drag a finger on the table; the racket sits slightly ahead of a finger so it stays visible. Arrow keys and WASD also work. Shots cross the net and bounce before a return. Contact near the racket’s edge angles your return; longer rallies become faster.

Matches are first to 11, with a two-point lead. Service changes every two points, then every point at deuce. Tap **Serve** between points; **Space** serves, pauses, or resumes. Backgrounding or resizing pauses the rally without advancing unseen play. Short landscape screens rotate the court. Normal play renders at the display cadence; power saver keeps the court black and uses fewer renders while preserving the same physics. Ready, paused, and point screens stop redrawing, including with the FPS counter enabled.

Rules and CPU logic live in `src/table_tennis.rs`; the shared vector renderer and pointer controls live in `src/table_tennis_view.rs`. Tests cover service, deuce, net clearance, bounces, misses, return timing, difficulty, interruptions, pointer bounds, display-independent simulation, and responsive geometry.

## Sudoku

Choose **Single player → Sudoku**, or open `?game=sudoku`. Generate a new
**Classic, Killer, V & X, Kropki, Thermo, or Diagonal** puzzle at **Easy, Medium,
or Hard**, or resume the last saved game. The board and number pad take their
layout cues from [SudokuPad / Cracking the Cryptic](https://sudokupad.com/).

Every grid is original and generated offline. The generator removes clues only
when its constraint-aware solver can still finish using logical deductions;
this proves one solution without guessing. Easy uses singles and variant
constraints; Medium adds locked candidates; Hard adds naked pairs and removes
more starting digits. Difficulty is algorithmic rather than a human play-test
rating. Killer cages cover the grid, stay connected and never repeat a digit.
XV and Kropki use **positive clues only**: unmarked edges have no extra rule.
Thermometers increase strictly from bulb to tip; Diagonal adds both diagonals.
The in-game rules explain each mode. Generation proceeds over input-responsive
frames, then the page returns to event-driven idle rendering.

Tap a cell, then a number, or drag to select a group. The four tools enter large
digits, corner notes, centre notes, or one of nine colours. **Space** cycles
tools; **Z / X / C / V** select them directly. **Shift + digit** adds corner
notes; **Ctrl + digit** adds centre notes. Arrows move selection; Shift/Ctrl
arrows extend it. **Ctrl + A** selects the whole grid. Delete/Backspace/0 erases
the current tool. **Ctrl + Z / Y** undo/redo a complete action.

**Check (K)** highlights incorrect digits; **Hint (H)** explains and enters the
next logically forced digit, or points out a contradiction to clear. Notes do
not constrain the hint solver. Givens cannot be changed. Reset is confirmed
and undoable. A full grid wins only if every Sudoku and variant rule holds.
Pinch or scroll over the board to zoom, drag a zoomed board to pan, and use
**1:1** to fit it again. Zoom keeps native display resolution.

The current puzzle, marks, colours and undo/redo history persist in browser
storage or `sudoku.json` in the OS app-data folder. Malformed or ambiguous
saves are rejected. Rules and generation live in `src/sudoku.rs` and
`src/sudoku/`; the responsive UI is in `src/sudoku_view.rs`.
