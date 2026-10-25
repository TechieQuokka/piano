# piano Tutorial (Korean)

Same IDs and `Expect` lines as `docs/TUTORIAL.md`. Titles and field names stay in
English; descriptions and hints are Korean.

## SCREEN {#screen}

    화면은 네 영역으로 나뉨:
      Keys (위)        피아노 건반, 항상 보임
      Code (왼쪽)      작성 중인 조각, 줄 번호 포함
      Display (오른쪽) 결과, 도움말, 설명서, 튜토리얼 ...
      Cmd (아래)       play, save, help 같은 명령어
    F12로 영역을 옮기고 (반대 방향: Shift+F12),
    F2 (Keys), F3 (Code), F4 (Cmd)로 바로 이동.
    영역을 옮겨도 Display는 그대로이고, 명령으로만 바뀜
    (played, ref, output, song, help, man, tutorial).
    Try      F12를 몇 번 누르고, F2를 누르기.
    Hint     맨 아래 상태줄에 지금 포커스가 보임.

## KEYS {#keys}

    Keys 포커스에서는 키보드가 소리를 들어 보는 피아노가 됨.
    Z X C V B N M은 흰 건반, S D G H J는 검은 건반.
    소리만 나고 코드에는 아무것도 적히지 않음.
    Try      Z X C 누르기.
    Hint     누른 건반이 빛남. Cmd `played`로 C4 도  D4 레  E4 미 목록을 봄 (돌아오기: `tutorial`).

## OCTAVE SHIFT {#shift}

    윗줄 Q W E R T Y U는 한 옥타브 높음.
    ← →는 건반 범위 전체를 한 옥타브씩 옮김.
    범위 양옆의 흐린 건반과 상태줄로 지금 위치를 알 수 있음.
    Try      Q를 누르고, →를 누른 뒤 Z 누르기.
    Hint     → 뒤에는 Z가 C5를 냄 (`played`에 C5 C5가 보임).

## CODE {#code}

    들어 본 음을 남기려면 Code 포커스(F3)에서 직접 적음.
    음은 이름(C D E F G A B), 선택 옥타브(C4),
    선택 길이(/8)로 이루어짐. 바뀐 것만 적음:
    /8은 한 번만 적고 다음 음은 이어받음.
    Try      F3을 누르고 C4/8 D E F G/4 입력하기.
    Expect   C4/8 D E F G/4
    Hint     D E F는 /8을 이어받음. man length에 길이 설명이 있음.

## REST AND CHORD {#chord}

    r은 쉼표. { } 안의 음은 화음으로 같이 울림.
    Try      C4 r {C E G} 입력하기.
    Expect   C4 r {C E G}
    Hint     man rest, man chord.

## PLAY {#play}

    F5는 커서 위치부터 재생, F6은 정지.
    Cmd(F4)에서: play, play 2-4 (줄), loop 1-2, stop.
    Try      F4를 누르고 play를 입력한 뒤 Enter.
    Hint     소리가 안 나면 Cmd `output`으로 이유를 확인: 사운드폰트가 없을 수 있음 (font).

## NEAREST OCTAVE {#nearest}

    숫자가 없으면 바로 앞 음과 가장 가까운 옥타브가 됨.
    그래서 C D E F G A B C는 다음 C까지 올라감.
    Try      C4 D E F G A B C 입력하기.
    Expect   C4 D E F G A B C5
    Hint     Cmd `ref`로 커서 위치의 옥타브 기준을 봄.

## DYNAMICS {#dynamics}

    p(여리게) ... f(세게)는 그 지점부터 소리 크기를 바꿈.
    cresc는 다음 기호까지 점점 세짐.
    Try      p cresc C4 D E F f G 입력하기.
    Expect   C4 D E F G
    Hint     man dynamic, man cresc.

## PEDALS {#pedals}

    ped on ... ped off는 모든 음을 계속 울리게 함.
    sos on은 이미 울리던 음만 유지함.
    soft on은 소리를 부드럽게 함.
    Try      ped on C3/2 E G ped off 입력하기.
    Expect   C3/2 E G
    Hint     Cmd `ped on`도 커서 위치에 같은 글자를 넣음.

## REPEATS {#repeats}

    |: ... :|는 구간을 두 번, x3을 붙이면 세 번 연주함.
    | C D E F | % |는 바로 앞 마디를 반복함.
    Try      |: C4 E G :| 입력하기.
    Expect   C4 E G C4 E G
    Hint     man repeat, man volta.

## FUNCTIONS {#functions}

    내장 함수는 구간을 바꿈: up, down, accent,
    louder, softer, tuplet, seq. 중첩할 수 있음.
    Try      up(12){ C4 E G } 입력하기.
    Expect   C5 E G
    Hint     Cmd `ref`로 모든 함수와 설명을 봄.

## SONG {#song}

    프로젝트에는 조각들(p1.piano, p2.piano ...)과 song.piano가 있음.
    song.piano에는 조각 순서를 적음: p1 > p2 > p1.
    Try      Cmd에서: new p2, 그다음 open song 하고 p1 > p2 쓰기.
    Hint     play song은 곡 구성을 재생함. man connector.

## SAVE AND EXPORT {#save}

    save는 조각을 저장. export midi / export wav는 파일로 내보냄.
    저장 안 한 작업은 복구본에 남음 (recover).
    Try      Cmd에서: save, 그다음 export midi.
    Hint     help export에 옵션이 있음.

## NEXT {#next}

    기본을 다 익혔음. 설명서에 모든 규칙이 있음:
      man            목차
      man octave     한 절
      lang ko        한글 설명
    Try      Cmd에서 man 입력하기.
    Hint     tutorial 1로 처음부터 다시 시작.
