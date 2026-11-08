"""Records the walkthrough of the browser version of swarmy: Firefox on the headless display :7 of
headless.sh (2000x1600, the page at 2x, 1000x800), driven through Marionette by the addresses of
the views and their elements (no screen coordinates), recorded by ffmpeg. Saves the screenshots of
the README on the way.

    scripts/walkthrough/headless.sh start
    python3 scripts/walkthrough/tour.py http://corvid-zt:7878 target/walkthrough
    scripts/walkthrough/headless.sh stop

The administrator key, to show Scan, is read from $SWARMY_ADMIN_KEY and is never typed on screen.
"""

import base64, json, os, socket, subprocess, sys, tempfile, time

SERVER, OUT = sys.argv[1].rstrip("/"), sys.argv[2]
os.makedirs(OUT, exist_ok=True)
PORT = 2840
profile = tempfile.mkdtemp(prefix="tour-profile-")
with open(os.path.join(profile, "user.js"), "w") as prefs:
    for pref, value in {
        "marionette.port": PORT,
        "layout.css.devPixelsPerPx": '"2"',
        "browser.shell.checkDefaultBrowser": "false",
        "browser.startup.homepage_override.mstone": '"ignore"',
        "datareporting.policy.dataSubmissionEnabled": "false",
        "toolkit.telemetry.reportingpolicy.firstRun": "false",
        "browser.aboutwelcome.enabled": "false",
        "ui.prefersReducedMotion": "0",
    }.items():
        prefs.write(f"user_pref({json.dumps(pref)}, {value});\n")
env = dict(os.environ, DISPLAY=":7", MOZ_ENABLE_WAYLAND="0")
env.pop("WAYLAND_DISPLAY", None)
firefox = subprocess.Popen(
    ["firefox", "-no-remote", "-marionette", "-kiosk", "-profile", profile, "about:blank"],
    env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(60):
    try:
        sock = socket.create_connection(("127.0.0.1", PORT)); break
    except OSError:
        time.sleep(0.5)
reader = sock.makefile("rb")


def packet():
    n = b""
    while (c := reader.read(1)) != b":":
        n += c
    return json.loads(reader.read(int(n)))


packet()
mid = 0


def cmd(name, params=None):
    global mid
    mid += 1
    body = json.dumps([0, mid, name, params or {}]).encode()
    sock.sendall(str(len(body)).encode() + b":" + body)
    reply = packet()
    if reply[2]:
        raise RuntimeError(f"{name}: {reply[2]}")
    return reply[3]


def js(script):
    return cmd("WebDriver:ExecuteScript", {"script": script, "args": []})["value"]


START = time.time()
chapters = []


def chapter(title):
    chapters.append(f"{int(time.time() - START)} {title}")
    print(chapters[-1], flush=True)


def view(route, wait=5):
    """Opens the view of an address, i.e. `#/replay-list`, the page itself is not reloaded."""
    js(f'window.location.hash = {json.dumps(route)}; return 1;')
    time.sleep(wait)


def scroll(pixels, steps=6, pause=0.35):
    for _ in range(steps):
        js(f'document.querySelector("#swarmy-tauri-content").scrollBy({{top: {pixels // steps}, behavior: "smooth"}}); return 1;')
        time.sleep(pause)
    time.sleep(1.5)


def top():
    js('document.querySelector("#swarmy-tauri-content").scrollTo({top: 0}); return 1;')
    time.sleep(0.8)


def click_text(selector, pattern, wait=2.5):
    found = js(f"""const el = [...document.querySelectorAll({json.dumps(selector)})]
        .find(e => new RegExp({json.dumps(pattern)}).test(e.textContent.trim()));
      if (!el) return false; el.scrollIntoView({{block: "center", behavior: "smooth"}});
      setTimeout(() => el.click(), 400); return true;""")
    time.sleep(wait)
    return found


def snap(name):
    image = cmd("WebDriver:TakeScreenshot", {"full": False})["value"]
    with open(os.path.join(OUT, f"{name}.png"), "wb") as png:
        png.write(base64.b64decode(image))


cmd("WebDriver:NewSession", {"capabilities": {"timeouts": {"script": 300000, "pageLoad": 300000}}})
# Without a window manager the kiosk window keeps its default size: the whole display.
cmd("WebDriver:SetWindowRect", {"x": 0, "y": 0, "width": 1000, "height": 800})
cmd("WebDriver:Navigate", {"url": f"{SERVER}/#/home"})
time.sleep(3)
key = os.environ.get("SWARMY_ADMIN_KEY", "")
if key:
    # The key of the tab, as Config does, never shown.
    js(f'sessionStorage.setItem("swarmy-admin-key", {json.dumps(key)}); return 1;')
    cmd("WebDriver:Navigate", {"url": f"{SERVER}/#/home"})
    time.sleep(3)

recorder = subprocess.Popen(
    ["ffmpeg", "-loglevel", "error", "-y", "-f", "x11grab", "-video_size", "2000x1600",
     "-framerate", "20", "-draw_mouse", "0", "-i", ":7", "-c:v", "libx264", "-preset", "veryfast",
     "-crf", "18", "-pix_fmt", "yuv420p", os.path.join(OUT, "raw.mp4")])
time.sleep(1)
START = time.time()

chapter("Home: the snapshot and where it comes from")
view("#/home", 4); snap("home"); time.sleep(2)

chapter("Config: the administrator key, the theme, Battle.net")
view("#/config", 4); snap("config"); time.sleep(2)

if key:
    chapter("Scan: the replays and the snapshot of the server")
    view("#/scan", 6); snap("scan"); scroll(500); time.sleep(1)

chapter("Map List")
view("#/map-list", 5); snap("map_list"); scroll(600); time.sleep(1)

chapter("Map Details: the minimap, the matchups, the resources, the lessons")
view("#/map-details?map=Winter%20Madness%20LE", 6); snap("map_details"); scroll(700); scroll(700); time.sleep(1)

chapter("Map Scout, experimental: where the first units go")
view("#/map-scout?map=Winter%20Madness%20LE", 3)
click_text("button", "^Scout$", 7); snap("map_scout")

chapter("Replay List: the games, their best rank")
view("#/replay-list", 6); snap("replay_list"); scroll(500); time.sleep(1)

chapter("Replay Detail: Reynor against Serral on Rainfall")
view("#/replay-detail?replay=36179", 8); snap("replay_detail")
scroll(900); time.sleep(1.5); scroll(900); snap("replay_detail_lessons"); time.sleep(1.5)
scroll(1400); scroll(900); snap("replay_detail_charts")
click_text("button[aria-pressed]", "^Army", 3)
click_text("button[aria-pressed]", "^Macro", 3)
scroll(1200); time.sleep(1.5); scroll(1200); time.sleep(1.5)

chapter("Player: Serral, the accounts, Battle.net, the insights")
view("#/player?name=Serral", 9); snap("player"); scroll(700); snap("player_battlenet"); scroll(900); scroll(900); scroll(900); time.sleep(1)

chapter("Patch Overview: the changes of each patch")
view("#/patch-overview", 6); snap("patch_overview"); click_text("button", "^Zerg$", 3); scroll(600)

chapter("Patch Evolution: a unit over the patches, the abilities")
view("#/patch-evolution", 6); snap("patch_evolution"); scroll(900)
click_text("button[aria-pressed]", "Fungal Growth", 3); click_text("button[aria-pressed]", "Transfuse", 3)
snap("patch_evolution_abilities")

chapter("Unit Overview: the race, then the unit")
view("#/unit-overview?unit=Marine", 5); snap("unit_overview")
click_text("button", "^Zerg$", 3); scroll(500)

chapter("Ability: Fungal Growth")
view("#/ability?ability=FungalGrowth.Execute", 6); snap("ability"); scroll(700); time.sleep(1)

chapter("Race Detail")
view("#/race-detail?race=Protoss", 6); snap("race_detail"); scroll(800); time.sleep(1)

chapter("Help and Changelog")
view("#/help", 3); scroll(800); view("#/changelog", 3); time.sleep(2)

recorder.terminate()
recorder.wait(timeout=30)
with open(os.path.join(OUT, "chapters.txt"), "w") as out:
    out.write("\n".join(chapters) + "\n")
cmd("WebDriver:DeleteSession")
firefox.terminate()
print("tour done", int(time.time() - START), "s")
