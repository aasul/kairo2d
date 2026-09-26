"""Regenerate the original pixel art and synthesized sounds for Relay Dusk.

Requires Pillow for PNG generation; WAV generation uses the standard library.
The generated files are checked in so players do not need Python.
"""

from __future__ import annotations

import math
from pathlib import Path
import random
import struct
import wave

from PIL import Image, ImageDraw


OUT = Path(__file__).parent / "assets"
OUT.mkdir(exist_ok=True)


def sprite(name: str, size: int, draw_fn) -> None:
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(image))
    image.save(OUT / f"{name}.png")


def player(d):
    d.polygon([(24, 2), (41, 36), (31, 31), (24, 39), (17, 31), (7, 36)], fill="#081e30")
    d.polygon([(24, 5), (36, 34), (24, 28), (12, 34)], fill="#3ee9d2")
    d.polygon([(24, 11), (29, 26), (24, 22), (19, 26)], fill="#e7fff3")
    d.polygon([(17, 31), (24, 40), (31, 31), (24, 35)], fill="#ffae5a")
    d.line([(24, 5), (24, 24)], fill="#ffffff", width=2)


def hunter(d):
    d.polygon([(24, 4), (43, 19), (38, 39), (24, 44), (10, 39), (5, 19)], fill="#260f26")
    d.polygon([(24, 8), (38, 21), (34, 35), (24, 40), (14, 35), (10, 21)], fill="#f05a85")
    d.rectangle((17, 20, 21, 25), fill="#fff4d9")
    d.rectangle((27, 20, 31, 25), fill="#fff4d9")
    d.polygon([(20, 35), (24, 42), (28, 35)], fill="#71253d")


def sniper(d):
    d.polygon([(24, 3), (42, 24), (24, 45), (6, 24)], fill="#23162f")
    d.polygon([(24, 8), (38, 24), (24, 40), (10, 24)], fill="#b887ff")
    d.polygon([(24, 14), (32, 24), (24, 34), (16, 24)], fill="#2c1641")
    d.rectangle((22, 19, 26, 29), fill="#fff2ff")


def tank(d):
    d.rectangle((5, 8, 42, 41), fill="#37212a")
    d.rectangle((9, 12, 38, 37), fill="#ec9869")
    d.rectangle((14, 16, 33, 33), fill="#6d3340")
    d.rectangle((20, 20, 27, 29), fill="#ffe9a7")
    for x in (4, 39):
        d.rectangle((x, 14, x + 5, 22), fill="#ffcb78")
        d.rectangle((x, 28, x + 5, 36), fill="#ffcb78")


def mote(d):
    d.polygon([(24, 5), (43, 24), (24, 43), (5, 24)], fill="#143535")
    d.polygon([(24, 10), (38, 24), (24, 38), (10, 24)], fill="#6be0a4")
    d.polygon([(24, 17), (31, 24), (24, 31), (17, 24)], fill="#e4ffe3")


def boss(d):
    d.polygon([(48, 6), (77, 18), (91, 48), (77, 78), (48, 91), (19, 78), (5, 48), (19, 18)], fill="#1c102b")
    d.polygon([(48, 12), (72, 23), (84, 48), (72, 72), (48, 84), (24, 72), (12, 48), (24, 23)], fill="#a34de2")
    d.polygon([(48, 20), (68, 29), (76, 48), (68, 67), (48, 76), (28, 67), (20, 48), (28, 29)], fill="#281336")
    d.polygon([(48, 29), (67, 48), (48, 67), (29, 48)], fill="#f3b5ff")
    d.polygon([(48, 36), (60, 48), (48, 60), (36, 48)], fill="#ffffff")
    for x, y in ((15, 15), (72, 15), (15, 72), (72, 72)):
        d.rectangle((x, y, x + 8, y + 8), fill="#f36f9f")


def core(d):
    d.polygon([(12, 1), (22, 12), (12, 23), (2, 12)], fill="#0f5555")
    d.polygon([(12, 4), (19, 12), (12, 20), (5, 12)], fill="#72fff0")
    d.rectangle((10, 10, 14, 14), fill="#ffffff")


for name, size, maker in (
    ("player", 48, player), ("hunter", 48, hunter), ("sniper", 48, sniper),
    ("tank", 48, tank), ("mote", 48, mote), ("boss", 96, boss),
    ("core", 24, core),
):
    sprite(name, size, maker)


def sound(name: str, duration: float, frequency: float, sweep: float = 0.0,
          noise: float = 0.0, volume: float = 0.28) -> None:
    rate = 22050
    count = int(duration * rate)
    rng = random.Random(name)
    samples = bytearray()
    phase = 0.0
    for i in range(count):
        t = i / rate
        phase += 2 * math.pi * max(30, frequency + sweep * t) / rate
        attack = min(1.0, t * 90)
        release = max(0.0, 1.0 - t / duration) ** 2
        value = (math.sin(phase) * 0.7 + math.sin(phase * 2.02) * 0.18)
        value += noise * (rng.random() * 2 - 1)
        samples.extend(struct.pack("<h", int(max(-1, min(1, value * attack * release * volume)) * 32767)))
    with wave.open(str(OUT / f"{name}.wav"), "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(rate)
        wav.writeframes(samples)


sound("shot", 0.12, 540, -2600, 0.16, 0.20)
sound("hit", 0.24, 180, -520, 0.65, 0.24)
sound("pickup", 0.28, 570, 1450, 0.02, 0.20)
sound("relay", 0.85, 210, 770, 0.03, 0.26)
sound("dash", 0.24, 280, -620, 0.18, 0.18)
sound("boss", 1.2, 105, -50, 0.30, 0.28)
sound("victory", 1.5, 340, 260, 0.02, 0.30)

# A quiet looping harmonic bed; no recorded or third-party audio is used.
rate, duration = 22050, 8.0
samples = bytearray()
notes = (110.0, 138.59, 164.81, 130.81)
for i in range(int(rate * duration)):
    t = i / rate
    fade = min(1.0, t * 3, (duration - t) * 3)
    note = notes[int(t * 2) % len(notes)]
    tone = math.sin(2 * math.pi * note * t) * 0.45
    tone += math.sin(2 * math.pi * note * 2 * t) * 0.13
    tone += math.sin(2 * math.pi * 55 * t) * 0.26
    samples.extend(struct.pack("<h", int(tone * fade * 4500)))
with wave.open(str(OUT / "drone.wav"), "wb") as wav:
    wav.setnchannels(1)
    wav.setsampwidth(2)
    wav.setframerate(rate)
    wav.writeframes(samples)
