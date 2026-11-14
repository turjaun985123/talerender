# Film validation — run record (seat W20)

- **Date:** 2026-11-14 (Asia/Dhaka)
- **Validator:** `tools/film/validate_film.py` (exit code 1)
- **Manifest:** `scenes/film/film.json` — 14 segments, 30 fps, 1811 frames,
  frame ranges tile [0, 1811) exactly; caption ranges tile chunks 0..65
  exactly once (per manifest assignment by chunk start time).
- **Segment files present at run time (8/14):**
  s01, s02, s03, s04, s05a, s05b, s06, s07
- **Segment files missing at run time (6/14) — Batch 2 seats still landing,
  reported here, not hidden:**
  s08, s08b, s09, s10, s11, s12
- **Headline:** 3/7 checks PASS — version (8/8 files), attachments
  (0 top-level violations under §14.5 R2; 6 attachments on character nodes),
  durations (frame tiling exact, 60.357 s -> 1811 frames). Failures:
  6 segment files missing; 307 of 704 numbers in lights/camera/character
  data do not round-trip as f32 (plain decimal literals, e.g. `2.6`);
  caption chunks 31 and 39 are each listed by two segment files and
  chunks 42..65 are not yet covered; all 8 present files declare
  atmosphere preset names ('noon'/'fog'/'golden') rather than filmLooks
  keys (the derived (set, preset) mapping resolves 14/14). Every failure
  is listed verbatim in the output below.

## Validator output (verbatim)

```
validate_film.py — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER (14 segments, 30 fps, 1811 frames)

[1 parse] FAIL — 8 present, 6 missing, 0 unparsable
    present: 8/14 segment files (s01, s02, s03, s04, s05a, s05b, s06, s07)
    missing: s08, s08b, s09, s10, s11, s12

[2 version] PASS — 8 files checked, 0 wrong

[3 f32] FAIL — 704 numbers checked, 307 do not round-trip as f32
    scenes/film/s01.scene.json: camera.eye[0] = 0.85 -> f32 0.8500000238418579
    scenes/film/s01.scene.json: camera.eye[1] = 2.7 -> f32 2.700000047683716
    scenes/film/s01.scene.json: camera.eye[2] = 5.4 -> f32 5.400000095367432
    scenes/film/s01.scene.json: camera.keys[0].eye[0] = 0.85 -> f32 0.8500000238418579
    scenes/film/s01.scene.json: camera.keys[0].eye[1] = 2.7 -> f32 2.700000047683716
    scenes/film/s01.scene.json: camera.keys[0].eye[2] = 5.4 -> f32 5.400000095367432
    scenes/film/s01.scene.json: camera.keys[0].target[0] = 0.35 -> f32 0.3499999940395355
    scenes/film/s01.scene.json: camera.keys[0].target[1] = 1.12 -> f32 1.1200000047683716
    scenes/film/s01.scene.json: camera.keys[0].target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s01.scene.json: camera.keys[1].eye[0] = 0.55 -> f32 0.550000011920929
    scenes/film/s01.scene.json: camera.keys[1].eye[1] = 1.62 -> f32 1.6200000047683716
    scenes/film/s01.scene.json: camera.keys[1].eye[2] = 3.2 -> f32 3.200000047683716
    scenes/film/s01.scene.json: camera.keys[1].target[0] = 0.35 -> f32 0.3499999940395355
    scenes/film/s01.scene.json: camera.keys[1].target[1] = 1.02 -> f32 1.0199999809265137
    scenes/film/s01.scene.json: camera.keys[1].target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s01.scene.json: camera.target[0] = 0.35 -> f32 0.3499999940395355
    scenes/film/s01.scene.json: camera.target[1] = 1.12 -> f32 1.1200000047683716
    scenes/film/s01.scene.json: camera.target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s01.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s01.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s01.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s01.scene.json: lights.points[0].decay = 1.6 -> f32 1.600000023841858
    scenes/film/s01.scene.json: lights.points[0].intensity = 4.6 -> f32 4.599999904632568
    scenes/film/s01.scene.json: lights.points[0].position[0] = -1.35 -> f32 -1.350000023841858
    scenes/film/s01.scene.json: lights.points[0].position[1] = 0.6 -> f32 0.6000000238418579
    scenes/film/s01.scene.json: lights.points[0].position[2] = -2.45 -> f32 -2.450000047683716
    scenes/film/s01.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.blink.at[0].t = 4.2 -> f32 4.199999809265137
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.camPitch.divisor = 0.9 -> f32 0.8999999761581421
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[0][1] = -0.45 -> f32 -0.44999998807907104
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[1][0] = 2.6 -> f32 2.5999999046325684
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[1][1] = 0.4 -> f32 0.4000000059604645
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[1][2] = 2.2 -> f32 2.200000047683716
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[2][0] = 5.2 -> f32 5.199999809265137
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[2][1] = -0.2 -> f32 -0.20000000298023224
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').channels.headTurn.keys[2][2] = 2.2 -> f32 2.200000047683716
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').position[0] = 0.35 -> f32 0.3499999940395355
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').position[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s01.scene.json: nodes[80] (character 'Onoda (S01)').state.headTurn = -0.45 -> f32 -0.44999998807907104
    scenes/film/s02.scene.json: camera.eye[1] = 3.1 -> f32 3.0999999046325684
    scenes/film/s02.scene.json: camera.eye[2] = 2.4 -> f32 2.4000000953674316
    scenes/film/s02.scene.json: camera.keys[0].eye[1] = 3.1 -> f32 3.0999999046325684
    scenes/film/s02.scene.json: camera.keys[0].eye[2] = 2.4 -> f32 2.4000000953674316
    scenes/film/s02.scene.json: camera.keys[0].t = 5.907 -> f32 5.9070000648498535
    scenes/film/s02.scene.json: camera.keys[0].target[0] = -0.1 -> f32 -0.10000000149011612
    scenes/film/s02.scene.json: camera.keys[0].target[2] = -5.2 -> f32 -5.199999809265137
    scenes/film/s02.scene.json: camera.keys[1].eye[0] = 0.9 -> f32 0.8999999761581421
    scenes/film/s02.scene.json: camera.keys[1].eye[1] = 1.85 -> f32 1.850000023841858
    scenes/film/s02.scene.json: camera.keys[1].eye[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s02.scene.json: camera.keys[1].t = 13.061 -> f32 13.060999870300293
    scenes/film/s02.scene.json: camera.keys[1].target[1] = 1.15 -> f32 1.149999976158142
    scenes/film/s02.scene.json: camera.keys[1].target[2] = -4.2 -> f32 -4.199999809265137
    scenes/film/s02.scene.json: camera.near = 0.05 -> f32 0.05000000074505806
    scenes/film/s02.scene.json: camera.target[0] = -0.1 -> f32 -0.10000000149011612
    scenes/film/s02.scene.json: camera.target[2] = -5.2 -> f32 -5.199999809265137
    scenes/film/s02.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s02.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s02.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s02.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s02.scene.json: nodes[49] (character 'onoda').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s02.scene.json: nodes[49] (character 'onoda').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s02.scene.json: nodes[49] (character 'onoda').position[2] = -7.6 -> f32 -7.599999904632568
    scenes/film/s02.scene.json: nodes[50] (character 'compA').position[0] = -1.4 -> f32 -1.399999976158142
    scenes/film/s02.scene.json: nodes[51] (character 'compB').position[0] = 1.4 -> f32 1.399999976158142
    scenes/film/s02.scene.json: nodes[51] (character 'compB').position[2] = -9.1 -> f32 -9.100000381469727
    scenes/film/s03.scene.json: camera.eye[0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s03.scene.json: camera.eye[2] = 8.4 -> f32 8.399999618530273
    scenes/film/s03.scene.json: camera.keys[0].eye[0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s03.scene.json: camera.keys[0].eye[2] = 8.4 -> f32 8.399999618530273
    scenes/film/s03.scene.json: camera.keys[0].target[1] = 1.22 -> f32 1.2200000286102295
    scenes/film/s03.scene.json: camera.keys[0].target[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s03.scene.json: camera.keys[1].eye[0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s03.scene.json: camera.keys[1].eye[2] = 8.4 -> f32 8.399999618530273
    scenes/film/s03.scene.json: camera.keys[1].t = 0.2889 -> f32 0.2888999879360199
    scenes/film/s03.scene.json: camera.keys[1].target[1] = 1.22 -> f32 1.2200000286102295
    scenes/film/s03.scene.json: camera.keys[1].target[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s03.scene.json: camera.keys[2].eye[0] = 0.05 -> f32 0.05000000074505806
    scenes/film/s03.scene.json: camera.keys[2].eye[1] = 1.85 -> f32 1.850000023841858
    scenes/film/s03.scene.json: camera.keys[2].eye[2] = 7.2 -> f32 7.199999809265137
    scenes/film/s03.scene.json: camera.keys[2].t = 2.6001 -> f32 2.600100040435791
    scenes/film/s03.scene.json: camera.keys[2].target[1] = 1.22 -> f32 1.2200000286102295
    scenes/film/s03.scene.json: camera.keys[2].target[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s03.scene.json: camera.keys[3].eye[0] = 0.05 -> f32 0.05000000074505806
    scenes/film/s03.scene.json: camera.keys[3].eye[1] = 1.85 -> f32 1.850000023841858
    scenes/film/s03.scene.json: camera.keys[3].eye[2] = 7.2 -> f32 7.199999809265137
    scenes/film/s03.scene.json: camera.keys[3].t = 2.889 -> f32 2.8889999389648438
    scenes/film/s03.scene.json: camera.keys[3].target[1] = 1.22 -> f32 1.2200000286102295
    scenes/film/s03.scene.json: camera.keys[3].target[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s03.scene.json: camera.target[1] = 1.22 -> f32 1.2200000286102295
    scenes/film/s03.scene.json: camera.target[2] = 0.6 -> f32 0.6000000238418579
    scenes/film/s03.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s03.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s03.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s03.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s03.scene.json: nodes[49] (character 'Taniguchi (shore)').channels.armR.keys[1][0] = 1.589 -> f32 1.5889999866485596
    scenes/film/s03.scene.json: nodes[49] (character 'Taniguchi (shore)').channels.armR.keys[1][2] = 0.3 -> f32 0.30000001192092896
    scenes/film/s03.scene.json: nodes[49] (character 'Taniguchi (shore)').position[0] = -1.05 -> f32 -1.0499999523162842
    scenes/film/s03.scene.json: nodes[49] (character 'Taniguchi (shore)').state.look[0] = 0.3 -> f32 0.30000001192092896
    scenes/film/s03.scene.json: nodes[50] (character 'Onoda (shore)').channels.blink.at[0].t = 14.78 -> f32 14.779999732971191
    scenes/film/s03.scene.json: nodes[51] (character 'companion A (shore)').position[0] = 1.85 -> f32 1.850000023841858
    scenes/film/s03.scene.json: nodes[51] (character 'companion A (shore)').position[2] = -0.85 -> f32 -0.8500000238418579
    scenes/film/s03.scene.json: nodes[52] (character 'companion B (shore)').position[0] = -1.9 -> f32 -1.899999976158142
    scenes/film/s03.scene.json: nodes[52] (character 'companion B (shore)').position[2] = -0.65 -> f32 -0.6499999761581421
    scenes/film/s04.scene.json: camera.eye[1] = 1.8 -> f32 1.7999999523162842
    scenes/film/s04.scene.json: camera.eye[2] = 7.6 -> f32 7.599999904632568
    scenes/film/s04.scene.json: camera.keys[0].eye[1] = 1.8 -> f32 1.7999999523162842
    scenes/film/s04.scene.json: camera.keys[0].eye[2] = 7.6 -> f32 7.599999904632568
    scenes/film/s04.scene.json: camera.keys[0].t = 15.95 -> f32 15.949999809265137
    scenes/film/s04.scene.json: camera.keys[0].target[0] = 0.8 -> f32 0.800000011920929
    scenes/film/s04.scene.json: camera.keys[0].target[1] = 1.1 -> f32 1.100000023841858
    scenes/film/s04.scene.json: camera.keys[0].target[2] = -0.6 -> f32 -0.6000000238418579
    scenes/film/s04.scene.json: camera.keys[1].eye[0] = 0.85 -> f32 0.8500000238418579
    scenes/film/s04.scene.json: camera.keys[1].eye[1] = 1.65 -> f32 1.649999976158142
    scenes/film/s04.scene.json: camera.keys[1].eye[2] = 6.6 -> f32 6.599999904632568
    scenes/film/s04.scene.json: camera.keys[1].t = 21.384 -> f32 21.384000778198242
    scenes/film/s04.scene.json: camera.keys[1].target[0] = 0.8 -> f32 0.800000011920929
    scenes/film/s04.scene.json: camera.keys[1].target[1] = 1.1 -> f32 1.100000023841858
    scenes/film/s04.scene.json: camera.keys[1].target[2] = -0.6 -> f32 -0.6000000238418579
    scenes/film/s04.scene.json: camera.near = 0.05 -> f32 0.05000000074505806
    scenes/film/s04.scene.json: camera.target[0] = 0.8 -> f32 0.800000011920929
    scenes/film/s04.scene.json: camera.target[1] = 1.1 -> f32 1.100000023841858
    scenes/film/s04.scene.json: camera.target[2] = -0.6 -> f32 -0.6000000238418579
    scenes/film/s04.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s04.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s04.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s04.scene.json: lights.points[0].decay = 1.6 -> f32 1.600000023841858
    scenes/film/s04.scene.json: lights.points[0].intensity = 4.6 -> f32 4.599999904632568
    scenes/film/s04.scene.json: lights.points[0].position[0] = -1.35 -> f32 -1.350000023841858
    scenes/film/s04.scene.json: lights.points[0].position[1] = 0.6 -> f32 0.6000000238418579
    scenes/film/s04.scene.json: lights.points[0].position[2] = -2.45 -> f32 -2.450000047683716
    scenes/film/s04.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s04.scene.json: nodes[80] (character 'compA').position[2] = -2.65 -> f32 -2.6500000953674316
    scenes/film/s04.scene.json: nodes[80] (character 'compA').state.bob = -3.9234 -> f32 -3.9233999252319336
    scenes/film/s04.scene.json: nodes[81] (character 'compB').position[0] = 0.85 -> f32 0.8500000238418579
    scenes/film/s04.scene.json: nodes[81] (character 'compB').position[2] = -2.95 -> f32 -2.950000047683716
    scenes/film/s04.scene.json: nodes[81] (character 'compB').state.bob = -3.4758 -> f32 -3.475800037384033
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.blink.maxOf[1].blinkAt.beats[0] = 20.7 -> f32 20.700000762939453
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[1][0] = 2.2 -> f32 2.200000047683716
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[1][1] = -0.35 -> f32 -0.3499999940395355
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[1][2] = 1.9 -> f32 1.899999976158142
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[2][0] = 4.6 -> f32 4.599999904632568
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[2][1] = 0.3 -> f32 0.30000001192092896
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').channels.headTurn.poseTrack.keys[2][2] = 1.9 -> f32 1.899999976158142
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').position[0] = 1.8 -> f32 1.7999999523162842
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').position[2] = 1.55 -> f32 1.5499999523162842
    scenes/film/s04.scene.json: nodes[82] (character 'onoda').state.headTurn = -0.349993 -> f32 -0.3499929904937744
    scenes/film/s05a.scene.json: camera.eye[0] = 2.4 -> f32 2.4000000953674316
    scenes/film/s05a.scene.json: camera.eye[2] = 4.2 -> f32 4.199999809265137
    scenes/film/s05a.scene.json: camera.keys[0].eye[0] = 2.4 -> f32 2.4000000953674316
    scenes/film/s05a.scene.json: camera.keys[0].eye[2] = 4.2 -> f32 4.199999809265137
    scenes/film/s05a.scene.json: camera.keys[0].t = 21.384 -> f32 21.384000778198242
    scenes/film/s05a.scene.json: camera.keys[0].target[1] = 6.4 -> f32 6.400000095367432
    scenes/film/s05a.scene.json: camera.keys[1].eye[1] = 1.2 -> f32 1.2000000476837158
    scenes/film/s05a.scene.json: camera.keys[1].eye[2] = 3.6 -> f32 3.5999999046325684
    scenes/film/s05a.scene.json: camera.keys[1].t = 24.56 -> f32 24.559999465942383
    scenes/film/s05a.scene.json: camera.near = 0.05 -> f32 0.05000000074505806
    scenes/film/s05a.scene.json: camera.target[1] = 6.4 -> f32 6.400000095367432
    scenes/film/s05a.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s05a.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s05a.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s05a.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s05b.scene.json: camera.eye[0] = 0.95 -> f32 0.949999988079071
    scenes/film/s05b.scene.json: camera.eye[2] = 2.6 -> f32 2.5999999046325684
    scenes/film/s05b.scene.json: camera.keys[0].eye[0] = 0.95 -> f32 0.949999988079071
    scenes/film/s05b.scene.json: camera.keys[0].eye[2] = 2.6 -> f32 2.5999999046325684
    scenes/film/s05b.scene.json: camera.keys[0].t = 24.56 -> f32 24.559999465942383
    scenes/film/s05b.scene.json: camera.keys[0].target[0] = 0.3 -> f32 0.30000001192092896
    scenes/film/s05b.scene.json: camera.keys[0].target[1] = 1.3 -> f32 1.2999999523162842
    scenes/film/s05b.scene.json: camera.keys[0].target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s05b.scene.json: camera.keys[1].eye[0] = 0.7 -> f32 0.699999988079071
    scenes/film/s05b.scene.json: camera.keys[1].eye[1] = 1.6 -> f32 1.600000023841858
    scenes/film/s05b.scene.json: camera.keys[1].eye[2] = 1.8 -> f32 1.7999999523162842
    scenes/film/s05b.scene.json: camera.keys[1].t = 26.129 -> f32 26.128999710083008
    scenes/film/s05b.scene.json: camera.keys[1].target[0] = 0.3 -> f32 0.30000001192092896
    scenes/film/s05b.scene.json: camera.keys[1].target[1] = 1.3 -> f32 1.2999999523162842
    scenes/film/s05b.scene.json: camera.keys[1].target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s05b.scene.json: camera.near = 0.05 -> f32 0.05000000074505806
    scenes/film/s05b.scene.json: camera.target[0] = 0.3 -> f32 0.30000001192092896
    scenes/film/s05b.scene.json: camera.target[1] = 1.3 -> f32 1.2999999523162842
    scenes/film/s05b.scene.json: camera.target[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s05b.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s05b.scene.json: lights.key.intensity = 2.6 -> f32 2.5999999046325684
    scenes/film/s05b.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s05b.scene.json: lights.points[0].decay = 1.6 -> f32 1.600000023841858
    scenes/film/s05b.scene.json: lights.points[0].intensity = 4.968072 -> f32 4.968071937561035
    scenes/film/s05b.scene.json: lights.points[0].position[0] = -1.35 -> f32 -1.350000023841858
    scenes/film/s05b.scene.json: lights.points[0].position[1] = 0.6 -> f32 0.6000000238418579
    scenes/film/s05b.scene.json: lights.points[0].position[2] = -2.45 -> f32 -2.450000047683716
    scenes/film/s05b.scene.json: lights.rim.intensity = 0.4 -> f32 0.4000000059604645
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').channels.armL.poseTrack.keys[1][0] = 1.59 -> f32 1.590000033378601
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').channels.armL.poseTrack.keys[1][2] = 0.45 -> f32 0.44999998807907104
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').position[0] = 0.35 -> f32 0.3499999940395355
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').position[2] = -0.55 -> f32 -0.550000011920929
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').state.headTurn = 0.04 -> f32 0.03999999910593033
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').state.look[0] = 0.1 -> f32 0.10000000149011612
    scenes/film/s05b.scene.json: nodes[80] (character 'onoda').state.look[1] = 0.3 -> f32 0.30000001192092896
    scenes/film/s06.scene.json: camera.keys[0].eye[0] = 1.7 -> f32 1.7000000476837158
    scenes/film/s06.scene.json: camera.keys[0].eye[1] = 1.9 -> f32 1.899999976158142
    scenes/film/s06.scene.json: camera.keys[0].eye[2] = 4.4 -> f32 4.400000095367432
    scenes/film/s06.scene.json: camera.keys[0].target[0] = -0.3 -> f32 -0.30000001192092896
    scenes/film/s06.scene.json: camera.keys[0].target[1] = 1.05 -> f32 1.0499999523162842
    scenes/film/s06.scene.json: camera.keys[0].target[2] = -3.2 -> f32 -3.200000047683716
    scenes/film/s06.scene.json: camera.keys[1].eye[0] = 1.15 -> f32 1.149999976158142
    scenes/film/s06.scene.json: camera.keys[1].eye[1] = 1.68 -> f32 1.6799999475479126
    scenes/film/s06.scene.json: camera.keys[1].eye[2] = 2.9 -> f32 2.9000000953674316
    scenes/film/s06.scene.json: camera.keys[1].target[0] = -0.3 -> f32 -0.30000001192092896
    scenes/film/s06.scene.json: camera.keys[1].target[1] = 1.05 -> f32 1.0499999523162842
    scenes/film/s06.scene.json: camera.keys[1].target[2] = -3.2 -> f32 -3.200000047683716
    scenes/film/s06.scene.json: camera.near = 0.05 -> f32 0.05000000074505806
    scenes/film/s06.scene.json: camera.target[0] = -0.3 -> f32 -0.30000001192092896
    scenes/film/s06.scene.json: camera.target[1] = 1.05 -> f32 1.0499999523162842
    scenes/film/s06.scene.json: camera.target[2] = -3.2 -> f32 -3.200000047683716
    scenes/film/s06.scene.json: lights.hemisphere.intensity = 0.9 -> f32 0.8999999761581421
    scenes/film/s06.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s06.scene.json: lights.key.intensity = 0.7 -> f32 0.699999988079071
    scenes/film/s06.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s06.scene.json: lights.points[0].decay = 1.6 -> f32 1.600000023841858
    scenes/film/s06.scene.json: lights.points[0].position[0] = -1.35 -> f32 -1.350000023841858
    scenes/film/s06.scene.json: lights.points[0].position[1] = 0.6 -> f32 0.6000000238418579
    scenes/film/s06.scene.json: lights.points[0].position[2] = -2.45 -> f32 -2.450000047683716
    scenes/film/s06.scene.json: lights.rim.intensity = 0.15 -> f32 0.15000000596046448
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.position.keys[0][0] = 0.1 -> f32 0.10000000149011612
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.position.keys[0][1][0] = -1.7 -> f32 -1.7000000476837158
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.position.keys[1][0] = 3.55 -> f32 3.549999952316284
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.walk.distKeys[0][0] = 0.1 -> f32 0.10000000149011612
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.walk.distKeys[1][0] = 3.55 -> f32 3.549999952316284
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.walk.distKeys[1][1] = 5.96 -> f32 5.960000038146973
    scenes/film/s06.scene.json: nodes[80] (character 'compA').channels.walk.stride = 0.62 -> f32 0.6200000047683716
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[0][0] = 0.1 -> f32 0.10000000149011612
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[0][1][0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[0][1][2] = -5.6 -> f32 -5.599999904632568
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[1][0] = 3.55 -> f32 3.549999952316284
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[1][1][0] = -2.6 -> f32 -2.5999999046325684
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.position.keys[1][1][2] = -10.9 -> f32 -10.899999618530273
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.walk.distKeys[0][0] = 0.1 -> f32 0.10000000149011612
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.walk.distKeys[1][0] = 3.55 -> f32 3.549999952316284
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.walk.distKeys[1][1] = 6.1 -> f32 6.099999904632568
    scenes/film/s06.scene.json: nodes[81] (character 'compB').channels.walk.stride = 0.62 -> f32 0.6200000047683716
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[0][1][2] = -1.48 -> f32 -1.4800000190734863
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[1][0] = 3.4 -> f32 3.4000000953674316
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[1][1][2] = -1.48 -> f32 -1.4800000190734863
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[2][0] = 3.8 -> f32 3.799999952316284
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.position.keys[2][1][2] = -1.76 -> f32 -1.7599999904632568
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.sit.at = 3.55 -> f32 3.549999952316284
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').channels.sit.sitOn = 0.34 -> f32 0.3400000035762787
    scenes/film/s06.scene.json: nodes[82] (character 'onoda').state.sitOn = 0.34 -> f32 0.3400000035762787
    scenes/film/s07.scene.json: camera.eye[0] = 1.4 -> f32 1.399999976158142
    scenes/film/s07.scene.json: camera.eye[1] = 1.6 -> f32 1.600000023841858
    scenes/film/s07.scene.json: camera.eye[2] = 3.4 -> f32 3.4000000953674316
    scenes/film/s07.scene.json: camera.keys[0].eye[0] = 1.4 -> f32 1.399999976158142
    scenes/film/s07.scene.json: camera.keys[0].eye[1] = 1.6 -> f32 1.600000023841858
    scenes/film/s07.scene.json: camera.keys[0].eye[2] = 3.4 -> f32 3.4000000953674316
    scenes/film/s07.scene.json: camera.keys[0].target[0] = 1.4 -> f32 1.399999976158142
    scenes/film/s07.scene.json: camera.keys[0].target[2] = -1.8 -> f32 -1.7999999523162842
    scenes/film/s07.scene.json: camera.keys[1].eye[0] = 3.6 -> f32 3.5999999046325684
    scenes/film/s07.scene.json: camera.keys[1].eye[1] = 5.4 -> f32 5.400000095367432
    scenes/film/s07.scene.json: camera.keys[1].eye[2] = 7.2 -> f32 7.199999809265137
    scenes/film/s07.scene.json: camera.keys[1].t = 3.819 -> f32 3.819000005722046
    scenes/film/s07.scene.json: camera.keys[1].target[0] = 1.8 -> f32 1.7999999523162842
    scenes/film/s07.scene.json: camera.keys[1].target[1] = 0.8 -> f32 0.800000011920929
    scenes/film/s07.scene.json: camera.keys[1].target[2] = -2.4 -> f32 -2.4000000953674316
    scenes/film/s07.scene.json: camera.target[0] = 1.4 -> f32 1.399999976158142
    scenes/film/s07.scene.json: camera.target[2] = -1.8 -> f32 -1.7999999523162842
    scenes/film/s07.scene.json: lights.key.bias = -0.0006 -> f32 -0.0006000000284984708
    scenes/film/s07.scene.json: lights.key.intensity = 2.3 -> f32 2.299999952316284
    scenes/film/s07.scene.json: lights.key.normalBias = 0.02 -> f32 0.019999999552965164
    scenes/film/s07.scene.json: lights.points[0].decay = 1.6 -> f32 1.600000023841858
    scenes/film/s07.scene.json: lights.points[0].intensity = 4.6 -> f32 4.599999904632568
    scenes/film/s07.scene.json: lights.points[0].position[0] = -1.35 -> f32 -1.350000023841858
    scenes/film/s07.scene.json: lights.points[0].position[1] = 0.6 -> f32 0.6000000238418579
    scenes/film/s07.scene.json: lights.points[0].position[2] = -2.45 -> f32 -2.450000047683716
    scenes/film/s07.scene.json: lights.rim.intensity = 0.6 -> f32 0.6000000238418579
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').attachments[0].offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').attachments[0].offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.armR.keys[1][0] = 1.91 -> f32 1.909999966621399
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.armR.keys[1][2] = 0.3 -> f32 0.30000001192092896
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.armR.keys[2][0] = 2.86 -> f32 2.859999895095825
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.camPitch.atEnd = 0.4726 -> f32 0.4726000130176544
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.camPitch.atStart = 0.1108 -> f32 0.11079999804496765
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.camPitch.divisor = 0.9 -> f32 0.8999999761581421
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.headTilt.keys[1][0] = 1.2 -> f32 1.2000000476837158
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.holdL.at = 1.96 -> f32 1.9600000381469727
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.holdL.atFilm = 32.35 -> f32 32.349998474121094
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.mood.keys[1][0] = 1.2 -> f32 1.2000000476837158
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.phase.stride = 0.6 -> f32 0.6000000238418579
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.position.from[2] = -1.76 -> f32 -1.7599999904632568
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.position.segment[0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.position.segment[1] = 1.85 -> f32 1.850000023841858
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.position.to[0] = 2.2 -> f32 2.200000047683716
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').channels.position.to[2] = -2.3 -> f32 -2.299999952316284
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').envTint.fogMix = 0.06 -> f32 0.05999999865889549
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').envTint.sat = 1.02 -> f32 1.0199999809265137
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').envTint.warm = 0.08 -> f32 0.07999999821186066
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').position[2] = -1.76 -> f32 -1.7599999904632568
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').rifleNote.offset[1] = 0.03 -> f32 0.029999999329447746
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').rifleNote.offset[2] = 0.07 -> f32 0.07000000029802322
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').state.camPitch = 0.1108 -> f32 0.11079999804496765
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').state.look[0] = 0.2 -> f32 0.20000000298023224
    scenes/film/s07.scene.json: nodes[78] (character 'Onoda').state.look[1] = -0.1 -> f32 -0.10000000149011612

[4 attachments] PASS — 0 top-level violations, 6 attachments on character nodes

[5 captions] FAIL — 44 chunk listings, 42 unique chunks covered of 66, 2 duplicates, 24 uncovered
    chunk 31 listed 2 times (in s05b, s06)
    chunk 39 listed 2 times (in s06, s07)
    chunks not covered by any present segment file: 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65

[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames (total 1811); tiling exact

[7 atmosphere] FAIL — 8 files checked, 8 declarations not filmLooks keys (strict); derived (set, preset) mapping resolves for 14/14 segments
    scenes/film/s01.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s02.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s03.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s04.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s05a.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s05b.scene.json: declared atmosphere 'noon' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s06.scene.json: declared atmosphere 'fog' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    scenes/film/s07.scene.json: declared atmosphere 'golden' is not a filmLooks key (keys: bk-noon-shop, jg-dusk, jg-fog-s06, jg-golden-s07, jg-noon, sh-noon)
    -- derived mapping (context) --
    s01: preset 'noon' on set JG -> filmLook 'jg-noon' exists
    s02: preset 'noon' on set SH -> filmLook 'sh-noon' exists
    s03: preset 'noon' on set SH -> filmLook 'sh-noon' exists
    s04: preset 'noon' on set JG -> filmLook 'jg-noon' exists
    s05a: preset 'noon' on set JG -> filmLook 'jg-noon' exists
    s05b: preset 'noon' on set JG -> filmLook 'jg-noon' exists
    s06: preset 'fog' on set JG -> filmLook 'jg-fog-s06' exists
    s07: preset 'golden' on set JG -> filmLook 'jg-golden-s07' exists

OVERALL: FAIL (3/7 checks passed)

```
