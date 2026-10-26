# piano Language Manual (Korean)

Same IDs and examples as `docs/LANGUAGE.md`. Titles and field names stay in English;
only descriptions are Korean.

## BASICS {#basics}

### FILE {#file}

    Syntax   <piece>.piano                  p1.piano  song.piano
    Flow     악보처럼 왼쪽에서 오른쪽, 위에서 아래로 읽음.
    Lines    줄바꿈은 자유. 한 줄에 1-2마디를 권장:
             재생 범위와 알림이 줄 번호를 씀.
    Errors   조각 전체가 멈추지 않음. 모르는 부분은 알림이 되고
             나머지는 그대로 재생됨.
    Example  C4 D E F
    See      comment, case

### COMMENT {#comment}

    Syntax   // text                        C D // intro
    Rule     // 뒤부터 줄 끝까지 무시됨.
    Note     #은 주석이 아니라 샵(반음 올림).
    Example  C D // intro
    See      file

### CASE {#case}

    Syntax   음 이름과 키워드는 대소문자를 구분하지 않음.   c d e  =  C D E
    Except   소문자 f 하나만 쓰면 세기 기호 forte(포르테).
             음 F는 F, f4, f/4처럼 씀.
    Space    공백은 토큰을 나눌 뿐, 개수는 상관없음.
    Example  c d e F f g
    See      name, dynamic

## NOTES {#notes}

### NAME {#name}

    Syntax   C D E F G A B                  C  D  E
    Meaning  국제 표준 음 이름 (C = 도, D = 레, ... B = 시).
    Example  C D E F G A B
    See      accidental, octave

### ACCIDENTAL {#accidental}

    Syntax   <name>#  <name>b               C#  Bb
    Meaning  #은 반음 올림(위의 검은 건반), b는 반음 내림.
    Same     C#과 Db는 같은 건반.
    Parse    첫 글자는 음 이름, 바로 뒤의 b는 플랫.
             bb = B플랫 (한 음). b b = B 두 개.
    Spell    어느 쪽으로 써도 됨. piano import는 악보의 표기를 따름.
    Example  C C# D Eb E
    See      name

### OCTAVE {#octave}

    Syntax   <note><0-8>                    C4  A0  C8
    Range    A0 - C8 (88건반). C4 = 가운데 도. 숫자는 C에서 바뀜.
    Omitted  바로 앞 음과 가장 가까운 옥타브. 시작 기준은 C4.
             거리가 같으면(6반음) 위로 가고 Warning.
    Resets   숫자를 직접 쓰면 그 음이 새 기준이 됨.
    Rests    쉼표는 기준을 바꾸지 않음.
    Chords   화음의 첫 음이 기준이 됨.
    Folding  A0-C8 밖이면 범위 안으로 옥타브를 옮기고 Warning.
    Example  C4 D E F G A B C   → 마지막 C는 C5
    See      inheritance, fn.up, fn.down

### LENGTH {#length}

    Syntax   <note>/<n>   n = 1 2 4 8 16 32 64 128      C/8  G/2
    Meaning  /1 온음표, /2 2분음표, /4 4분음표, /8 8분음표, ...
             n이 클수록 짧음.
    Omitted  앞 음이나 쉼표의 길이를 이어받음. 시작은 /4.
    Invalid  다른 숫자(예: /3)는 Error, 길이는 그대로.
    Example  C4/8 D E F G/4   → D E F는 8분음표
    See      dot, tuplet, inheritance

### DOT {#dot}

    Syntax   /<n>.   /<n>..                 C/4.  C/4..
    Meaning  .은 길이의 절반을, ..은 절반 + 4분의 1을 더함.
             /4. = 1.5박, /4.. = 1.75박.
    Limit    /n 바로 뒤에만. 점은 2개까지 (더 많으면 Warning).
    Inherit  점도 길이와 함께 이어받음.
    Example  C/4. D E F/8 G   → D E는 점4분음표
    See      length

### TIE {#tie}

    Syntax   <note>~ <same note>            G/2~ G/4
    Meaning  같은 높이의 두 음을 하나의 긴 음으로 이음.
             두 번째 음은 다시 치지 않음.
    Bars     마디줄을 넘어도 되고, 함수 안팎으로도 이어짐.
    Chords   {C E G}/2~ {C E G}/4 는 세 음을 모두 이음.
             {C~ E G}/2 C/4 는 C만 이음: {} 안의 음 뒤에 붙인 ~.
    Other    ~ 뒤가 다른 음이면 Warning, 붙임줄은 무시됨.
    Example  C4 D G/2~ | G/4 F
    See      length, chord

### REST {#rest}

    Syntax   r   r/<n>                      r  r/8  r/2.
    Meaning  주어진 길이만큼 쉼.
    Inherit  이어받은 길이를 쓰고, 새 길이를 정하기도 함.
             옥타브 기준은 바꾸지 않음.
    Example  C4 r D r/8 E
    See      length, octave

### CHORD {#chord}

    Syntax   {<notes>}/<n>                  {C E G}/2
    Meaning  동시에 치는 음들.
    Length   } 뒤에 한 번만 씀. {} 안에 길이를 쓰면 Warning.
    Tie      } 뒤의 ~는 모든 음을, 안의 음 뒤의 ~는 그 음만 이음:
             {C3~ E4 G}/2 {C3 F4 A}/2
    Octave   화음 안의 각 음은 바로 앞 음 기준으로 해석됨.
             화음이 끝나면 첫 음이 기준이 됨.
    Example  {C4 E G}/2 {F A C}/2
    See      octave, tie, voices

## INHERITANCE {#inheritance}

### CONTINUITY {#continuity}

    Rule     값은 바뀔 때까지 다음 음으로 이어짐.
    Octave   앞 음과 가장 가까운 옥타브 (시작: C4)
    Length   앞 길이, 점 포함 (시작: /4)
    Dynamic  앞 세기 기호 (시작: mf)
    Pedals   켜고 끌 때까지 유지 (시작: 꺼짐)
    View     Cmd `ref`로 커서 위치의 상태를 봄.
    Example  C4/8 D E f F G   → D E는 C4 근처의 8분음표, F G는 포르테
    See      octave, length, dynamic, pedal

### DEFAULTS {#defaults}

    Values   tempo 100, time 4/4, length /4, dynamic mf, reference C4
    Change   piano.toml [defaults] 또는 ~/.config/piano/config.toml.
    Order    조각 머리말 > piano.toml > 전역 설정 > 내장 기본값
    Example  tempo 120 C D E
    See      tempo, time

## STRUCTURE {#structure}

### BAR {#bar}

    Syntax   |                              | C D E F |
    Meaning  마디줄. 선택 사항.
    Check    마디 길이가 박자와 다르면 Warning.
             첫 마디가 짧은 것(못갖춘마디)은 괜찮음.
             free 블록이 든 마디는 검사하지 않음.
    Example  | C4 D E F | G A B C |
    See      time, bar-repeat, free

### FREE {#free}

    Syntax   free{ ... }                    | free{ C/16 D E F G A } |
    Meaning  길이 제한이 없는, 박 없는 한 마디 (카덴차).
             그 마디는 박자표와 비교하지 않고, 다음 마디부터 다시
             검사함.
    Inside   | 말고는 무엇이든 (안에 | → Error). 적힌 길이대로
             연주하고, 상태와 붙임줄은 평소처럼 안팎으로 이어짐.
    Voices   성부마다 자기 마디를 검사함: free{ voices{ }{ } }는
             모든 성부의 마디를 풀어 줌.
    Example  time 3/4 | C4 E G | free{ G/16 A B C5 D E F G A } | C5/2. |
    See      bar, time

### TIME {#time}

    Syntax   time <beats>/<unit>            time 3/4  time 6/8
    Meaning  박자표. 마디 검사와 %에 쓰임.
    Where    어디든 쓸 수 있고, 그 지점부터 적용. 기본 4/4.
    Range    박 수 1-32, 단위 1 2 4 8 16 32 64 128. 그 밖은 Error.
    Example  time 3/4 | C4 D E | F G A |
    See      bar

### TEMPO {#tempo}

    Syntax   tempo <bpm>                    tempo 90
    Meaning  1분에 4분음표 몇 개 (20-400).
    Where    어디든 쓸 수 있고, 그 지점부터 적용. 기본 100.
    Range    20-400 밖은 Error, 템포는 그대로.
    Example  tempo 90 C D tempo 140 E F
    See      defaults

### VOICES {#voices}

    Syntax   voices{ ... }{ ... } ...       voices{ G4/2 A }{ C3/1 }
    Meaning  { } 하나가 성부 하나. 모든 성부가 함께 시작해 동시에
             울림: 화음 위의 멜로디, 그 아래의 긴 저음.
    State    각 성부는 블록 직전 상태에서 시작해 옥타브, 길이,
             셈여림, cresc를 따로 이어받음.
    After    블록 뒤에는 첫 번째 성부의 상태가 이어짐.
    Pedals   피아노 전체가 공유: 어느 성부의 페달이든 모든 성부에
             적용되고, 블록 뒤에도 그대로 유지됨.
    Bars     성부 안의 마디줄은 현재 마디에 이어서 셈.
    Length   성부 길이가 다르면 Warning. 블록 길이는 가장 긴 성부.
    Header   성부 안의 tempo, time -> Warning, 무시됨.
    Chord    블록 바로 뒤의 { }는 성부로 읽음. 뒤에 길이가 있으면
             화음: voices{ C }{ E } {C E G}/2
    Example  voices{ p r/2 G4/8. G4/16 G4/4 }{ pp {C3 C4}/1 }
    See      chord, pedal, bar

## DYNAMICS {#dynamics}

### DYNAMIC {#dynamic}

    Syntax   ppp pp p mp mf f ff fff        p C D f E
    Velocity ppp 16, pp 33, p 49, mp 64, mf 80, f 96, ff 112, fff 127
    Inherit  다음 기호까지 적용. 기본 mf.
    Example  p C4 D f E F
    See      cresc, case

### CRESC {#cresc}

    Syntax   cresc ... <mark>   dim ... <mark>        p cresc C D E F f
    Meaning  다음 세기 기호까지 음표마다 점점 세게(cresc)
             또는 점점 여리게(dim).
    Target   뒤에 세기 기호가 없으면 Warning.
             cresc인데 더 여린 기호로 가면(dim은 반대) Warning.
    Example  p cresc C4 D E F f G
    See      dynamic

## PEDALS {#pedals}

### PEDAL {#pedal}

    Syntax   ped on|off   sos on|off   soft on|off
    ped      댐퍼(오른쪽): 모든 음이 계속 울림.
    sos      소스테누토(가운데): 밟는 순간 울리던 음만 계속 울리고,
             그 뒤의 음은 평소처럼 끊김.
    soft     소프트/우나 코르다(왼쪽): 소리가 부드럽고 작아짐.
    Overlap  구간이 아니라 켜기/끄기라서 서로 자유롭게 겹쳐도 됨.
    Same     이미 켜진 페달을 또 켜면 Hint.
    Order    음 바로 앞에 쓴 페달은 그 음을 잡음:
             sos on C2/1 은 C2를 유지함.
    Example  sos on C2/1 sos off ped on C4 E G ped off
    See      continuity

## REPEATS {#repeats}

### REPEAT {#repeat}

    Syntax   |: ... :|   |: ... :| x<n>     |: C D :| x3
    Meaning  구간을 n번 연주 (기본 2). 매번 같은 소리가 남.
             옥타브는 글자 기준으로 한 번만 해석됨.
    Start    |: 없이 :|만 쓰면 처음(또는 앞 반복 뒤)부터 반복.
    Count    1-99. 그 밖은 Warning, 2로 처리.
    Example  |: C4 E G :| x3
    See      volta, bar-repeat

### VOLTA {#volta}

    Syntax   |: ... [1 ... :| [2 ... |      |: C D [1 E :| [2 G |
    Meaning  끝부분만 다르게: 1번째는 [1, 2번째는 [2, ...
    End      마지막 끝부분은 다음 | 또는 |: 또는 끝에서 멈춤.
    Example  |: C4 D [1 E :| [2 G | A
    See      repeat

### BAR-REPEAT {#bar-repeat}

    Syntax   %                              | C D E F | % |
    Meaning  바로 앞 마디를 그대로 반복.
    Needs    앞에 마디줄이 있어야 함. 앞 마디가 없으면 Warning.
    Example  | C4 D E F | % |
    See      bar, repeat

## FUNCTIONS {#fn}

### SYNTAX {#fn.syntax}

    Syntax   <name>(<args>){ ... }          up(12){ C E G }
    Builtin  내장 함수만 있음 (사용자 함수, 재귀 없음).
    Nesting  중첩 가능, 안쪽부터 바깥쪽으로 적용. 깊이 제한 32.
    Flow     옥타브는 함수 적용 전 글자 기준으로 해석되므로
             함수 뒤의 음은 영향을 받지 않음.
    Folding  결과가 A0-C8 밖이면 옥타브를 옮기고 Warning.
    Unknown  모르는 이름이나 잘못된 인자는 Error, 안의 음은 재생됨.
    Example  up(12){ accent(1/3){ C4/8 D E F G A } }
    See      fn.up, fn.accent

### UP {#fn.up}

    Syntax   up(<n>){ ... }                 up(12){ C E G }
    Meaning  모든 음을 n반음 올림 (12 = 한 옥타브).
    Range    n은 0-88. 더 크면 Error, 안의 음은 그대로 재생됨.
    Example  up(12){ C4 E G }
    See      fn.down, fn.seq

### DOWN {#fn.down}

    Syntax   down(<n>){ ... }               down(2){ G A B }
    Meaning  모든 음을 n반음 내림.
    Range    n은 0-88. 더 크면 Error, 안의 음은 그대로 재생됨.
    Example  down(2){ G4 A B }
    See      fn.up

### ACCENT {#fn.accent}

    Syntax   accent(1/<n>){ ... }   accent(<note>){ ... }
    Meaning  규칙에 맞는 음을 세게 함 (velocity +20).
    1/n      첫 음부터 n번째 타건(음이 시작되는 순간)마다. 화음은
             한 번으로 세고, 쉼표는 세지 않음. n은 1-64.
             accent(1/3){ C D E F G A } -> C와 F.
    note     그 이름의 모든 음: accent(G){ ... }.
    Example  accent(1/3){ C4 D E F G A }
    See      fn.louder

### LOUDER {#fn.louder}

    Syntax   louder(<n>){ ... }   softer(<n>){ ... }
    Meaning  모든 음의 세기에 n을 더하거나 뺌 (1-127 안으로 제한).
    Example  louder(20){ C4 D } softer(20){ E F }
    See      dynamic, fn.accent

### TUPLET {#fn.tuplet}

    Syntax   tuplet(<n>){ ... }             tuplet(3){ C/8 D E }
    Meaning  구간을 바로 아래 2의 거듭제곱 개수의 시간에 연주:
             3은 2개 시간에, 5는 4개, 7은 4개, 9와 11은 8개.
             n은 3-64. 3 5 6 7 9 10 11과 그 곱은 시간이 정확함.
    Example  tuplet(3){ C4/8 D E } F/4
    See      length

### SEQ {#fn.seq}

    Syntax   seq(<times>, <step>){ ... }    seq(3, 1){ C D E }
    Meaning  구간을 times번 연주하면서 매번 step반음씩 더 옮김
             (음악의 반복진행).
             seq(3, 1){ C D E } -> C D E, C# D# F, D E F#.
    Range    times는 1-64, step은 무엇이든 (음수면 내려감).
    Example  seq(3, 2){ C4 D E }
    See      fn.up, repeat

## SONG {#song}

### ARRANGEMENT {#arrangement}

    Syntax   <piece> > <piece> > ...        p1 > bridge1 > p4
    File     프로젝트 폴더의 song.piano.
    Pieces   각 조각은 독립적: 템포, 기본값, 페달이 따로임.
             같은 조각을 여러 번 써도 됨.
    Missing  없는 조각은 Error, 나머지는 재생됨.
    Example  p1 > p2 > p1
    See      connector

### CONNECTOR {#connector}

    Syntax   >   fade   r/<n>   ped         p1 > fade > p2
    >        바로 이어서.
    fade     앞 조각의 끝(마지막 마디)을 서서히 줄임.
    r/n      조각 사이에 쉼.
    ped      이어지는 부분에서 댐퍼 페달을 밟아 울림을 남김.
    Bridge   직접 만든 연결 조각도 그냥 일반 조각임.
    Example  p1 > fade > r/2 > p2 > ped > p3
    See      arrangement

### TRANSFORM {#transform}

    Syntax   <fn>(<args>){ <pieces> }       up(2){ p1 }
    Meaning  내장 함수를 조각에 적용. 원본 파일은 바뀌지 않음.
    Example  p1 > up(2){ p1 } > softer(20){ p2 }
    See      fn.syntax

## ALERTS {#alerts}

### LEVELS {#levels}

    ✖ Error    그 부분을 재생할 수 없음 (모르는 문법, 잘못된 길이).
    ⚠ Warning  재생은 되지만 의도와 다를 수 있음 (옥타브 접기,
               애매한 옥타브, 마디 길이, 무시된 붙임줄).
    · Hint     참고 정보 (예: |: 없는 :|, 이미 켜진 페달).
    Where      Output 화면에 줄 번호와 함께 표시. Code 줄 번호 옆에는
               그 줄에서 가장 심각한 알림 기호가 표시됨.
    Example  C4 F#   → Warning: 애매한 옥타브
    See      octave

## SUMMARY {#summary}

### GRAMMAR {#grammar}

    note     C D E F G A B  [# b]  [0-8]  [/n [. ..]]  [~]
    rest     r [/n [. ..]]
    chord    { note [~] ... } [/n] [~]
    comment  // ...
    header   tempo <bpm>   time <a>/<b>
    bar      |   %
    dynamic  ppp pp p mp mf f ff fff   cresc   dim
    pedal    ped|sos|soft on|off
    repeat   |: ... :| [x n]   [1 ... :| [2 ...
    function up down accent louder softer tuplet seq  (name(args){ ... })
    voices   voices{ ... }{ ... }
    free     free{ ... }
    song     piece > piece   fade   r/n   ped
    Example  tempo 90 | p C4/8 D E F G/4 A | ped on {C E G}/1 ped off |
    See      file
