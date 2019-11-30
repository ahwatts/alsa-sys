bindgen \
    --whitelist-function "^snd_.*" \
    --whitelist-var "^SND_.*" \
    --whitelist-var "^IEC958_.*" \
    --whitelist-var "^MIDI_.*" \
    --whitelist-type "^snd_.*" \
    --blacklist-type "pollfd" \
    $@
