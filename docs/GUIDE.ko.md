# piano Language Guide (Korean)

piano 언어의 자세한 설명서. 앱에서는 `guide`로 봄 (짧은 요약은 `man`,
`docs/LANGUAGE.ko.md`).

`docs/GUIDE.md`와 ID, 코드가 같아야 함 (코드의 주석은 달라도 됨). 제목, 소제목,
키워드, 코드는 영어로 두고 설명만 한국어로 씀.

## 1. Getting started {#start}

이 설명서는 piano 언어를 한 단계씩 설명합니다. 각 항목마다 음악에서 무슨 뜻인지,
어떻게 쓰는지, 정확한 규칙, Code에 그대로 따라 쓸 수 있는 예시, 자주 하는 실수를
다룹니다. `guide next`로 순서대로 읽거나, `guide <항목>`으로 한 주제로 바로 갈 수
있습니다 (예: `guide octave`). 같은 주제의 짧은 요약은 `man`에 있습니다.

### Your first piece {#first}

#### Concept

조각(piece)은 텍스트 파일입니다. 그 안의 단어 하나하나가 음이거나 지시이고,
편집기는 위에서 아래로 실행되는 프로그램처럼 단어를 차례대로 연주합니다. 쓰지 않은
값은 기본값으로 채워지기 때문에, 글자 몇 개만으로도 이미 음악이 됩니다.

#### Try it

F3을 눌러 Code로 가서 이렇게 써 보세요:

```piano
C D E F G
```

F5를 누르면 커서 위치부터, Cmd(F4)에서 `play`를 치면 처음부터 연주합니다.
가운데 도에서 시작해 올라가는 다섯 음이, 한 박씩, 보통 세기로 들립니다.

#### What was filled in

위 줄은 아래 줄을 줄여 쓴 것입니다. 각 값은 해당 항목에서 자세히 설명하니, 지금은
이런 값들이 있고 바꿀 수 있다는 것만 알면 됩니다.

```piano
tempo 100 time 4/4 mf C4/4 D E F G   // 같은 조각을 모두 적은 것
```

- `tempo 100`: 1분에 100박 (`guide tempo`).
- `time 4/4`: 한 마디에 4박 (`guide time`).
- `mf`: 조금 세게 (`guide dynamic`).
- `C4`: 건반 가운데의 도 (`guide octave`).
- `/4`: 각 음은 한 박, 4분음표 (`guide length`).

#### See also

`guide file` · `guide continuity` · `guide grammar`

### Pieces and files {#file}

#### Concept

프로젝트는 폴더 하나입니다. 그 안에 `piano.toml`(설정), 조각마다 파일 하나
(`p1.piano`, `verse.piano`, ...), 그리고 조각들의 순서를 정하는 `song.piano`가
있습니다. 조각은 인트로나 후렴처럼 음악의 한 부분입니다. 조각 이름은 `.piano`를 뺀
파일 이름이고, `new <이름>`으로 새로 만듭니다.

#### Rules

- 코드는 악보처럼 왼쪽에서 오른쪽, 위에서 아래로 읽습니다.
- 줄바꿈과 공백은 단어를 나눌 뿐입니다. 새 줄은 새 마디가 아니고, 음악을 멈추지도
  않습니다.
- 한 줄에 한두 마디씩 쓰세요. 조각의 일부만 연주할 때(`play 3`, `play 2-4`,
  `loop 2-4`)와 알림이 문제 위치를 가리킬 때 줄 번호를 씁니다.
- 실수가 있어도 조각은 멈추지 않습니다. 이해할 수 없는 부분은 알림이 되고
  (`guide levels`), 나머지는 그대로 연주됩니다.

#### Examples

```piano
// p1: 한 줄에 두 마디
| C4 D E F | G A B C |
| C B A G  | F E D C |
```

아래의 `$`는 언어에 없는 기호입니다. 오류로 알리고 건너뛰며, C, D, E는 그대로
연주됩니다.

```piano error
C D $ E   // Error: 모르는 기호, 음은 그대로 연주됨
```

#### Common mistakes

- 조각 전체를 한 줄에 쓰기. 연주는 되지만 `play 3`으로 고를 줄이 없고, 모든 알림이
  1번 줄을 가리킵니다.
- 줄바꿈이 마디를 끝낸다고 생각하기. 마디는 `|`로만 표시합니다 (`guide bar`).

#### See also

`guide comment` · `guide bar` · `guide levels`

### Comments {#comment}

#### Concept

주석은 편집기가 무시하는, 나를 위한 메모입니다. C나 Rust의 `//`와 같습니다. 조각의
부분에 이름을 붙이거나, 어떤 줄을 잠시 소리 나지 않게 할 때 씁니다.

#### Syntax

`//`부터 그 줄 끝까지가 주석입니다. 여러 줄 주석은 없습니다.

#### Examples

```piano
// 1절: 멜로디만
C4 D E F   // 올라감
// G A B C   (주석 처리: 연주되지 않음)
```

#### Common mistakes

- `#`은 주석이 아닙니다. 샵 기호이고, 음 이름 바로 뒤에서만 쓰입니다.

```piano error
# intro   // Error: #은 주석이 아니라 샵
C D E
```

- `/` 하나도 주석이 아닙니다. `C/8`처럼 길이를 시작합니다.

#### See also

`guide file` · `guide accidental`

### Upper and lower case {#case}

#### Rules

- 음 이름과 키워드는 대소문자를 가리지 않습니다: `c`는 `C`, `TEMPO 90`은
  `tempo 90`, `PED ON`은 `ped on`, `R`은 쉼표입니다.
- 예외 하나: 소문자 `f` 하나만 쓰면 세기 기호 포르테(세게)입니다. 음 F는 `F`,
  `f4`, `f/4`로 씁니다.
- 플랫 기호는 항상 소문자 `b`입니다: `Eb`, `eb`. `EB`는 모르는 단어가 됩니다.
- 단어 사이에는 공백이 필요합니다: `CDE`는 모르는 단어 하나입니다. `C D E`로
  쓰세요. 공백 개수는 상관없습니다.
- 길이, 점, 붙임줄은 공백 없이 음에 붙여 씁니다: `C4/8.~`.

#### Examples

```piano
c d e F f g   // c d e F는 음, f는 포르테, g는 음
```

```piano
C D E f G   // 음은 네 개: f는 G를 세게 할 뿐
```

```piano error
CDE   // Error: 모르는 단어 하나, C D E로 쓸 것
```

#### See also

`guide name` · `guide dynamic`

## 2. Pitch {#pitch}

음높이는 소리가 얼마나 높은지 낮은지입니다. 이 장에서는 건반, 음 이름, 샵과 플랫,
그리고 `C`라고만 썼을 때 어느 도인지 정하는 옥타브 규칙을 설명합니다.

### Keys and semitones {#semitone}

#### Concept

피아노에는 건반이 88개 있습니다. 왼쪽이 낮고 오른쪽이 높습니다. 건반은 흰 건반
7개와 검은 건반 5개, 모두 12개의 무늬가 되풀이됩니다. 한 건반에서 바로 옆 건반으로
(흰 건반이든 검은 건반이든) 가는 것이 반음, 가장 작은 걸음입니다. 반음 12개가
옥타브입니다. 거기 있는 음은 이름이 같고, 같은 음이 더 높게 들립니다.

코드로 생각하면: 음높이는 정수이고(MIDI처럼 C4 = 60), 반음은 +1, 옥타브는
+12입니다. `up(n)` 같은 함수는 이 정수에 더하기를 할 뿐입니다.

#### The pattern

```text
C  C#  D  D#  E  F  F#  G  G#  A  A#  B  C
0  1   2  3   4  5  6   7  8   9  10  11 12   semitones from C
```

- 검은 건반은 샵 음입니다: C#, D#, F#, G#, A#.
- 미–파(E–F)와 시–도(B–C) 사이에는 검은 건반이 없어서 반음 차이입니다. 나머지
  이웃한 흰 건반은 반음 두 개(온음) 차이입니다.

#### Try it

Keys(F2)에서는 키보드가 피아노가 됩니다. `Z S X D C V G B H N J M`을 누르면
C C# D D# E F F# G G# A A# B가 납니다. 이 순서로 쳐 보고 걸음이 고르게 올라가는
것을 들어 보세요.

#### See also

`guide name` · `guide accidental` · `guide octave`

### Note names {#name}

#### Concept

흰 건반 일곱 개는 알파벳으로 부릅니다: C D E F G A B, 곧 도 레 미 파 솔 라 시입니다.
B 다음에는 한 옥타브 위의 C부터 이름이 다시 시작됩니다.

#### Syntax

음은 알파벳 하나에, 필요하면 샵이나 플랫, 옥타브 숫자, 길이, 붙임줄을 이 순서로
붙입니다: `C`, `F#4`, `Bb3/8.`, `G/2~`.

#### Examples

```piano
C D E F G A B   // 도 레 미 파 솔 라 시
```

```piano
E4 D C D E E E/2   // 떴다 떴다 비행기
```

#### Common mistakes

- 계이름(도레미)과 독일식 H는 음 이름이 아닙니다.

```piano error
do re mi   // Error: C D E로 쓸 것
```

#### See also

`guide accidental` · `guide octave` · `guide case`

### Sharps and flats {#accidental}

#### Concept

샵 `#`은 음을 반음 올리고, 플랫 `b`는 반음 내립니다. 대개 검은 건반이 됩니다:
C#은 C 바로 오른쪽의 검은 건반입니다. C#과 Db는 같은 건반을 두 가지로 쓴 것이니,
읽기 편한 쪽을 쓰면 됩니다.

악보에는 조표가 있어서 곡 전체의 F, C, ...에 샵이나 플랫이 자동으로 붙습니다.
piano 언어에는 조표도 없고, 마디 끝까지 이어지는 임시표도 없습니다. 모든 음이 자기가
무엇인지 정확히 말합니다. 사장조(G major) 곡이라면 F#이어야 하는 F는 모두 `F#`으로
씁니다.

#### Rules

- `#`과 `b`는 알파벳 바로 뒤에 씁니다: `F#`, `Bb`, `F#4`, `Bb3/8`.
- 첫 글자가 음 이름이고, 바로 뒤의 `b`는 플랫입니다: `bb`는 B플랫 하나, `b b`는
  B 두 개입니다.
- E#은 F, Fb는 E, B#은 C, Cb는 B입니다. 샵과 플랫은 반음 움직일 뿐, 그 자리가
  어느 건반이든 상관없습니다.
- 제자리표는 없습니다. 그냥 알파벳이 제자리음입니다. 겹샵, 겹플랫도 없습니다.

#### Examples

```piano
C C# D Eb E   // Eb과 D#은 같은 건반
```

```piano
G A B C D E F# G   // 사장조: F는 모두 F#으로 씀
```

```piano
bb Bb b   // B플랫, B플랫, B
```

```piano error
C## D   // Error: 겹샵은 없음, D로 쓸 것
```

#### Common mistakes

- 조표가 있다고 생각하기: 사장조 멜로디에 `F`라고 쓰면 F#이 아니라 F가 납니다.
- 대문자 플랫: `EB`는 모르는 단어입니다. `Eb`로 쓰세요.

#### See also

`guide semitone` · `guide name`

### Octaves {#octave}

#### Concept

같은 음 이름이 모든 옥타브에 있으므로, 숫자로 어느 것인지 정합니다. `C4`는 건반
가운데쯤의 가운데 도입니다. 숫자는 C에서 하나씩 올라갑니다: B3은 C4 바로 아래
건반입니다. 가장 낮은 건반은 A0, 가장 높은 건반은 C8입니다.

#### Rules

- 숫자를 쓰면 정확히 그 건반입니다: `G4`, `Bb2`.
- 숫자를 안 쓰면, 바로 앞 음(기준음)과 가장 가까운 음이 됩니다. 조각을 시작할 때
  기준음은 C4입니다.
- 그래서 숫자 없이 쓸 수 있는 가장 큰 도약은 반음 6개입니다. 더 크게 뛰려면
  옥타브를 쓰세요.
- 정확히 반음 6개 떨어진 경우(C에서 F#, F에서 B, B에서 F)는 동점이라 위쪽을
  고르고 Warning을 냅니다. 숫자를 써서 분명히 하세요.
- 숫자가 있든 없든, 모든 음은 다음 음의 기준음이 됩니다.
- 쉼표는 기준음을 바꾸지 않습니다.
- 화음 안에서는 각 음이 화음 안의 바로 앞 음을 기준으로 정해지고, 화음이 끝나면
  첫 음이 기준음이 됩니다 (`guide chord`).
- A0–C8 밖의 음은 옥타브 단위로 범위 안으로 옮기고 Warning을 냅니다.
- `ref` 명령으로 커서 위치의 기준음을 볼 수 있습니다.

#### Examples

```piano
C4 D E F G A B C   // 마지막 C는 C5: B4에서 한 걸음 위
```

```piano
C4 E G C5 G4 E C   // 화음을 펼쳐서 올라갔다 내려옴
```

C4에서 G로 갈 때, 아래쪽 G(G3)는 반음 5개, 위쪽 G(G4)는 반음 7개 떨어져 있어서
`G`만 쓰면 아래로 갑니다:

```piano
C4 G    // G3: 더 가까운 G
C4 G4   // 위로 가려면 숫자를 씀
```

```piano
C4 C G4 G A A G/2   // 반짝반짝 작은 별: G4만 한 번 쓰면 나머지는 따라옴
```

```piano warning
C4 F#   // Warning: F#3, F#4 모두 반음 6개, F#4를 고름
```

```piano warning
C8 D   // Warning: D8은 없음, D7로 접힘
```

#### Common mistakes

- 반짝반짝 작은 별을 `C4 C G G A A G`로 쓰기: G가 G3으로 내려가 멜로디가
  뒤집힙니다. 숫자 하나로 고쳐집니다.
- 숫자 없이 멀리 뛰기. 헷갈리면 숫자를 쓰세요. 써서 손해 볼 일은 없습니다.

#### See also

`guide continuity` · `guide chord` · `guide fn.up`

## 3. Rhythm {#rhythm}

리듬은 음이 언제 시작해서 얼마나 길게 이어지는지입니다. 이 장에서는 음 길이, 점,
쉼표, 붙임줄, 템포, 박자표, 마디줄, 자유 마디, 잇단음표를 다루고, 마지막으로 값이 음에서 음으로
이어지는 방식을 설명합니다.

### Beats and lengths {#length}

#### Concept

음악은 시간을 박(beat)으로 셉니다. 4분음표가 한 박이고, 다른 길이는 그 절반이나
두 배입니다. `/` 뒤의 숫자는 온음표를 몇으로 나눴는지입니다: `/8`은 온음표의
8분의 1입니다. 숫자가 클수록 짧은 음입니다.

- `/1` 온음표: 4박
- `/2` 2분음표: 2박
- `/4` 4분음표: 1박
- `/8` 8분음표: 1/2박
- `/16` 16분음표: 1/4박
- `/32`, `/64`, `/128`: 1/8박, 1/16박, 1/32박

#### Rules

- 길이는 음(과 옥타브) 바로 뒤에 씁니다: `C/8`, `C4/8`, `{C E G}/2`, `r/8`.
- 길이를 안 쓰면 바로 앞 음이나 쉼표의 길이를 이어받습니다. 조각의 첫 음은
  4분음표입니다.
- 1, 2, 4, 8, 16, 32, 64, 128만 있습니다. 다른 숫자는 Error이고 길이는 바뀌지 않습니다.
  셋잇단음표는 `tuplet`으로 씁니다 (`guide fn.tuplet`).

#### Examples

```piano
C4/4 D E F   // 4분음표 넷: 4/4 한 마디
G/2 E/2      // 2분음표 둘
C/1          // 온음표 하나
```

```piano
C4/8 D E F G/4 E C/2   // 8분음표, 그다음 4분음표, 그다음 2분음표
```

```piano error
C/3   // Error: 3분음표는 없음, tuplet(3)을 쓸 것
```

#### Common mistakes

- 길이가 계속 이어진다는 것을 잊기:

```piano
C4/1 D E F   // 네 음 모두 온음표
```

#### See also

`guide dot` · `guide rest` · `guide fn.tuplet` · `guide continuity`

### Dotted notes {#dot}

#### Concept

점은 음을 원래 길이의 절반만큼 더 길게 합니다: 점4분음표는 1 + 1/2 = 1.5박입니다.
점을 하나 더 찍으면 길이의 4분의 1이 더해집니다: 겹점4분음표는 1.75박입니다.
점4분음표 다음 8분음표(길게, 짧게)는 두 박을 채우며, 음악에서 가장 흔한 리듬 중
하나입니다.

#### Rules

- 점은 길이 바로 뒤에 씁니다: `C/4.`, `C4/8..`. 길이 없는 점은 Error입니다.
- 점은 2개까지입니다. 더 많으면 Warning이고 2개로 처리합니다.
- 점도 길이와 함께 이어집니다: `C/4.` 다음 음들은 새 길이를 쓸 때까지
  점4분음표입니다.

#### Examples

```piano
C4/4. D/8 E/4. F/8   // 길게-짧게, 길게-짧게
```

```piano
C/2. D/4   // 3박 + 1박
```

```piano
C/4. D E F/8 G   // D, E도 점음표; F, G는 8분음표
```

```piano error
C. D   // Error: 점에는 길이가 필요함, C/4.로 쓸 것
```

```piano warning
C/4... D   // Warning: 점은 2개까지
```

#### See also

`guide length` · `guide tie`

### Rests {#rest}

#### Concept

쉼표는 일정한 길이 동안의 침묵입니다. 음악은 쉼표로 숨을 쉽니다.

#### Syntax

`r`, `r/<n>`, `r/<n>.` — 음높이가 없는 음처럼 씁니다.

#### Rules

- 쉼표도 음처럼 길이를 이어받고, 다음으로 넘겨줍니다.
- 쉼표는 옥타브 기준음을 바꾸지 않습니다.
- 쉼표는 붙임줄로 이을 수 없습니다.

#### Examples

```piano
C4 D E r   // 음 셋, 그리고 한 박 쉼
```

```piano
C4/8 r D r E/4 r   // 쉼표가 /8, 그다음 /4를 이어받음
```

```piano
B4 r C   // C5: 쉼표는 기준음을 옮기지 않음
```

```piano warning
C r/2~ r   // Warning: 쉼표는 붙임줄로 이을 수 없음
```

#### Common mistakes

- 쉼표의 길이도 다음 음으로 이어집니다:

```piano
C4/4 r/8 D   // D도 8분음표
```

#### See also

`guide length` · `guide octave`

### Ties {#tie}

#### Concept

붙임줄은 같은 높이의 두 음을 하나의 소리로 잇습니다. 길이가 더해지고 두 번째 음은
다시 치지 않습니다. 기호 하나로 쓸 수 없는 길이(5박 = 온음표 + 4분음표)나, 마디줄을
넘어 음을 끌 때 씁니다.

#### Syntax

`<음>~ <같은 음>`: `G/2~ G/4`.

#### Rules

- `~` 뒤의 음은 같은 건반이어야 합니다. 옥타브는 평소처럼 정해지므로 `G/2~ G`는
  숫자 없이도 됩니다.
- 붙임줄은 마디줄을 넘어갈 수 있습니다.
- 화음끼리도 이을 수 있습니다: 앞 화음의 각 음이 뒤 화음의 같은 음과 이어집니다.
- 화음 안에서 음 바로 뒤의 `~`는 그 음만 잇습니다: `{C4~ E G}/2 C`는 C를 이어 끌고
  E와 G는 끝냅니다. `}` 뒤의 `~`는 모든 음을 잇습니다.
- `~` 뒤에 다른 음, 쉼표, 또는 끝이 오면 Warning이고 붙임줄은 무시됩니다.
- 붙임줄은 함수 안팎으로 이어집니다. 예: `G~ tuplet(3){ G E C }`. 음은 함수를
  적용한 뒤의 높이로 맞추므로 `up(2){ C~ } D`는 D를 이어서 끕니다.
- 붙임줄은 반복과 `voices`의 안팎을 넘나들지 못합니다.
- 다른 음끼리 매끄럽게 잇는 이음줄(슬러)은 없습니다. 음을 이어서 울리게 하려면
  댐퍼 페달을 씁니다 (`guide pedal`).

#### Examples

```piano
C4/1~ C/4 D E F   // C가 5박 동안 이어짐
```

```piano
| C4 D E F~ | F/2 G/2 |   // F가 마디줄을 넘어 이어짐
```

```piano
{C4 E G}/2~ {C E G}/2   // 화음 전체가 4박 동안 이어짐
```

```piano
{C4~ E G}/2 {C F A}/2   // C가 4박 동안 이어지고, 그 위에 F와 A를 침
```

```piano warning
C4 D/2~ E   // Warning: D와 E는 다른 음, 붙임줄 무시
```

#### See also

`guide length` · `guide chord` · `guide bar`

### Tempo {#tempo}

#### Concept

템포는 빠르기, 곧 1분에 4분음표 박이 몇 번인지(BPM)입니다. 60이면 한 박이 1초,
120이면 0.5초입니다. 느린 곡은 60–76, 걷는 빠르기는 90–110, 빠른 곡은 120–160
정도입니다.

#### Syntax

`tempo <bpm>`, 20부터 400까지.

#### Rules

- 템포는 쓴 곳부터 적용됩니다. 조각 전체에 적용하려면 맨 위에 쓰세요. 쓰지 않으면
  기본값(100, `guide defaults`)을 씁니다.
- 박자표가 무엇이든 한 박은 항상 4분음표입니다.
- 점점 빠르게/느리게는 없습니다. 몇 단계로 나눠 바꾸세요.
- `voices` 안에서는 어느 성부에 쓴 tempo든 그 위치부터 모든 성부에 적용되고, 블록
  뒤에도 이어집니다 (`guide voices`).
- 곡(song)에서는 각 조각이 자기 템포로 시작합니다.

#### Examples

```piano
tempo 72 C4 E G C5/2   // 느리게
```

```piano
tempo 120 C4/8 D E F G A B C   // 8분음표 여덟 개가 2초
```

```piano
tempo 100 C4 D E F
tempo 90 G A B C
tempo 76 C/1   // 끝에서 단계적으로 느려짐
```

```piano error
tempo 500 C   // Error: tempo는 20-400
```

#### See also

`guide time` · `guide defaults`

### Time signatures {#time}

#### Concept

박자표는 박을 마디로 묶습니다. 위 숫자는 몇 개인지, 아래 숫자는 어떤 음표를 세는지
입니다: 4/4는 한 마디에 4분음표 넷, 3/4는 셋(왈츠), 2/4는 둘(행진곡), 6/8은
8분음표 여섯을 셋씩 두 묶음으로 느끼는 박자입니다.

#### Syntax

`time <박 수>/<단위>`: 박 수 1–32, 단위 1, 2, 4, 8, 16, 32, 64, 128.

#### Rules

- 쓴 곳부터 적용됩니다. 기본값은 4/4입니다.
- 쓰임은 두 가지뿐입니다: 마디 길이 검사(`guide bar`)와 `%`(`guide bar-repeat`).
  소리를 바꾸지 않고, 어떤 박도 저절로 강조하지 않습니다. 강조는 `accent`로 합니다
  (`guide fn.accent`).
- `voices` 안에서는 어느 성부에 쓴 time이든 그 위치부터 모든 성부에 적용되고, 블록
  뒤에도 이어집니다 (`guide voices`).
- `free` 블록이 든 마디는 박자표와 비교하지 않습니다 (`guide free`).

#### Examples

```piano
time 3/4 | C4 E G | C5/2. |   // 왈츠
```

```piano
time 6/8 | C4/8 D E F G A | G/4. E/4. |
```

```piano
| C4 D E F | time 3/4 G A B |   // 4/4, 그다음 3/4
```

```piano error
time 3/5 C   // Error: 단위는 1 2 4 8 16 32 64 128
```

#### See also

`guide bar` · `guide tempo`

### Bar lines {#bar}

#### Concept

마디는 박의 묶음 하나이고, `|`가 마디의 끝을 표시합니다. piano에서 마디줄은 선택
사항이고 소리를 바꾸지 않습니다. 코드의 assert처럼 동작해서, 마디 길이가 박자표와
맞지 않으면 Warning을 냅니다.

#### Rules

- 모든 마디를 박자표와 비교합니다. 짧거나 긴 마디는 Warning입니다.
- 첫 마디는 짧아도 됩니다: 첫 온전한 마디 앞의 못갖춘마디(여린내기)입니다.
  길면 안 됩니다.
- 마지막 마디도 짧아도 됩니다: 마지막 `|`가 닫는 마디로, 그 뒤에 시간이 흐르는 것이
  없을 때입니다. 못갖춘마디가 있으면 보통 그 나머지를 채웁니다. 길면 안 됩니다.
- `|`로 닫힌 마디만 검사합니다: 마지막 `|` 뒤의 부분은 박자표와 비교하지 않습니다.
- 조각 맨 앞의 `|`는 괜찮지만, 그러면 다음 마디는 못갖춘마디가 아니라 온전한
  마디로 셉니다.
- `%`는 앞 마디가 무엇인지 알기 위해 마디줄이 필요합니다.
- 줄바꿈은 마디줄이 아닙니다.
- `free` 블록이 든 마디는 검사하지 않습니다 (`guide free`).

#### Examples

```piano
| C4 D E F | G A B C |
```

```piano
G4 | C5 C C D | E/2 C/2 |   // 못갖춘마디: 첫 마디 앞에 한 박
```

```piano
G4 | C5 C C D | E/2 C/4 |   // 마지막 마디가 한 박 짧음: 못갖춘마디를 채움
```

```piano warning
| C4 D E | F G A B |   // Warning: 마디가 3박, 4/4는 4박
```

#### Common mistakes

- 못갖춘마디를 `|`로 시작하기: `| G4 | C5 ...`는 못갖춘마디를 짧은 온전한 마디로
  만들어 Warning이 납니다. 첫 `|`를 빼세요.

#### See also

`guide time` · `guide bar-repeat` · `guide file` · `guide free`

### Free bars {#free}

#### Concept

어떤 음악에는 박을 셀 수 없는 구간이 있습니다. 카덴차처럼, 연주자가 자유롭게 치는
작은 음표의 흐름입니다. `free{ }`는 그런 구간을 길이에 제한이 없는 한 마디로 씁니다.
그 한 마디만 마디 길이 assert를 끄는 셈입니다.

#### Syntax

`free{ <음표> }`: `free{ C/16 D E F G A B C }`.

#### Rules

- `free` 블록이 든 마디는 길이가 얼마든 박자표와 비교하지 않습니다. 그다음 마디부터
  다시 검사합니다.
- `free` 블록은 한 마디입니다. 안에 `|`를 쓰면 Error입니다. 마디줄은 블록 바깥에
  쓰세요.
- 그 밖의 것은 모두 안에 쓸 수 있습니다: 음표, 화음, 쉼표, 셈여림, 페달, 함수,
  `voices`.
- 적힌 길이를 현재 템포로 연주합니다. 소리는 바뀌지 않습니다.
- 옥타브, 길이, 셈여림은 평소처럼 블록 안팎으로 이어지고, 붙임줄도 블록 안으로,
  밖으로 이어집니다.
- `voices` 안에서는 성부마다 자기 마디를 검사하므로, 한 성부의 `free`는 그 성부의
  마디만 풀어 줍니다. 모든 성부의 마디를 풀려면 voices를 안에 넣으세요:
  `free{ voices{ ... }{ ... } }`.

#### Examples

```piano
time 3/4 | C4 E G | free{ G/16 A B C5 D E F G A B C6 B A G F E D C } | C5/2. |
// 흐름은 4박 반, 그 앞뒤 마디는 3/4
```

```piano
| free{ voices{ G5/32 A G A G A G A G A G A G A G A }{ {C3 E G}/2 } } |
// 누르고 있는 화음 위의 트릴, 4/4에서 두 박
```

```piano error
free{ C4 D | E F }   // Error: `free` 안에는 마디줄을 쓸 수 없음
```

#### Common mistakes

- 박을 잘못 센 것을 `free`로 감추기. 박이 있는 구간에서 길이가 틀린 마디는 대개
  오타이고, Warning은 그것을 찾으라고 있습니다.
- 긴 카덴차 안에 마디줄 쓰기. 빼거나, 블록을 여러 개로 나누세요:
  `| free{ ... } | free{ ... } |`.

#### See also

`guide bar` · `guide time` · `guide voices`

### Tuplets {#fn.tuplet}

#### Concept

셋잇단음표는 두 음의 시간에 세 음을 연주합니다. piano의 길이는 절반과 두 배뿐이라,
셋잇단음표는 `tuplet(3){ ... }` 안에 보통 길이로 쓰고 이것이 시간을 줄여 줍니다:
8분음표 셋이 8분음표 둘의 시간, 곧 한 박에 연주됩니다.

#### Syntax

`tuplet(<n>){ ... }`, n은 3부터 64까지.

#### Rules

- 안의 내용을 바로 아래 2의 거듭제곱 개수의 시간에 연주합니다: 3은 2에, 5는 4에,
  6은 4에, 7은 4에, 9는 8에, 11은 8에.
- 3, 5, 6, 7, 9, 10, 11과 이들로 만든 수(12, 14, 15, ...)는 시간이 정확합니다. 13처럼
  그 밖의 수는 가장 가까운 tick으로 반올림됩니다.
- 안의 모든 것이 줄어듭니다: 음, 쉼표, 화음.
- 블록 뒤에는 안에서 쓴 길이가 줄어들지 않은 채로 이어집니다. 다음 길이를 다시
  쓰세요.

#### Examples

```piano
tuplet(3){ C4/8 D E } F/4 G A   // 셋잇단 한 박, 그다음 4분음표 셋
```

```piano
| tuplet(3){ C4/4 E G } C5/2 |   // 4분음표 셋을 두 박에
```

```piano
tuplet(5){ C4/16 D E F G } A/4   // 16분음표 다섯을 한 박에
```

#### Common mistakes

- 블록 뒤의 길이를 잊기:

```piano
tuplet(3){ C4/8 D E } F G   // F, G는 8분음표: F/4로 쓸 것
```

#### See also

`guide length` · `guide fn.syntax`

### What carries over {#continuity}

#### Concept

piano는 기억합니다. 변수가 있는 프로그램처럼 상태를 가지고 있고, 모든 음이 그
상태를 읽고 바꿉니다. 값은 새로 쓸 때까지 유지됩니다. 그래서 `C4/8 D E`라고만 써도
D와 E가 C4 근처의 8분음표가 됩니다.

#### The state

- 옥타브 기준음: 바로 앞 음 (시작: C4).
- 길이(점 포함): 바로 앞 길이 (시작: `/4`).
- 세기: 마지막 세기 기호 (시작: `mf`).
- 페달: 바꿀 때까지 켜짐/꺼짐 유지 (시작: 모두 꺼짐).
- 템포, 박자표: 마지막으로 쓴 값 (시작: 100, 4/4).

시작 값은 프로젝트마다 바꿀 수 있습니다 (`guide defaults`). `ref` 명령은 Code
커서 위치의 상태를 보여 줍니다.

#### Where the flow branches

- 화음: 화음 뒤의 기준음은 화음의 첫 음입니다.
- 반복: 글자를 한 번만 읽으므로 매번 같은 소리가 납니다.
- 함수: 안의 내용은 변환 전에 쓴 그대로 읽고, 함수 뒤의 음은 변환된 음이 아니라
  마지막으로 쓴 음에서 이어집니다.
- `voices`: 각 성부는 블록 직전 상태에서 시작하고, 블록 뒤에는 첫 성부의 상태가
  이어집니다.
- 곡(song): 각 조각은 기본값에서 새로 시작합니다.

#### Examples

```piano
C4/8 D E f F G   // D E: C4 근처의 8분음표; F G: 8분음표, 포르테
```

```piano
up(12){ C4 D E } F   // F는 F4: 쓴 그대로의 E4를 따라감
```

#### Common mistakes

- 끝내려던 긴 길이나 센 세기가 계속 이어지는 것. 음이 이상하게 들리면 커서를 그
  음에 두고 `ref`를 실행해 보세요.

#### See also

`guide octave` · `guide length` · `guide dynamic` · `guide defaults`

## 4. Chords and voices {#harmony}

피아노 음악은 한 번에 한 음만 나오는 경우가 드뭅니다. 화음은 여러 건반을 함께 치고,
성부(voices)는 저음 위의 멜로디처럼 여러 줄기를 동시에 움직이게 합니다.

### Chords {#chord}

#### Concept

화음은 동시에 치는 건반들입니다. 가장 흔한 화음은 3도씩 쌓은 세 음, 3화음입니다.
예를 들어 C E G(다장조 화음, C major)입니다. piano에서 화음은 하나의 사건입니다.
모든 음이 함께 시작하고, 길이도 세기도 같습니다.

#### Syntax

`{<음들>}/<n>`: `{C E G}/2`, `{C4 E G}/2~`, `{C4~ E G}/2`.

#### Rules

- 길이는 `}` 뒤에 한 번만 씁니다. 안에 쓰면 Warning이고 무시됩니다.
- 중괄호 안의 각 음은 바로 앞 음을 기준으로 정해져서 `{C E G}`는 위로 쌓입니다.
  화음 뒤의 기준음은 첫 음입니다.
- 안에는 음만 올 수 있습니다: 쉼표, 기호, 길이는 안 됩니다. 안의 음에는 자기
  `~`를 붙일 수 있고, 그 음만 이어집니다 (`guide tie`).
- 화음도 음처럼 길이를 이어받고 넘겨줍니다.
- 같은 건반을 두 번 쓰면 한 번만 칩니다.

#### Examples

```piano
{C4 E G}/2 {F A C}/2 {G B D}/2 {C E G}/2
```

```piano
{C3 G3 C4 E}/1   // 넓은 화음: 크게 뛸 때는 숫자를 씀
```

```piano
C4/8 E G C5 {C4 E G C5}/2   // 펼쳐서, 그다음 한꺼번에
```

```piano warning
{C4/8 E G}   // Warning: 길이는 } 뒤에: {C4 E G}/8
```

```piano error
{C4 r G}/2   // Error: 화음 안에는 음만
```

#### Common mistakes

- 화음의 음마다 길이가 다르기를 기대하기. 예를 들어 움직이는 음 아래의 긴 저음은
  `voices`로 쓰거나, 끄는 음에 `~`를 붙입니다: `{C3~ E4 G}/2 {C3 F4 A}/2`.

#### See also

`guide voices` · `guide tie` · `guide octave`

### Voices {#voices}

#### Concept

한 성부는 긴 음을 끄는 동안 다른 성부가 움직이거나, 오른손 멜로디 아래에서 왼손이
베이스를 치는 경우가 많습니다. 화음은 음들이 길이를 공유하므로 이렇게 할 수
없습니다. `voices`는 여러 흐름을 동시에 실행합니다. 함께 시작해서 끝에서 합쳐지는
스레드와 같습니다.

#### Syntax

`voices{ ... }{ ... } ...`: 본문이 둘 이상이고, `{ }` 하나가 성부 하나입니다.

#### Rules

- 모든 성부가 같은 순간에 시작합니다.
- 각 성부는 블록 직전 상태에서 시작해 옥타브, 길이, 세기, cresc를 따로 가집니다.
- 블록 뒤에는 첫 번째 성부의 상태가 이어집니다.
- 페달은 피아노 전체의 것입니다: 어느 성부의 페달이든 모든 성부에 적용되고, 블록
  뒤에도 그대로 유지됩니다.
- 성부 안의 마디줄은 현재 마디에 이어서 셉니다.
- 성부 길이가 다르면 Warning이고, 블록은 가장 긴 성부만큼 이어집니다.
- `tempo`와 `time`은 피아노 전체의 것입니다: 어느 성부에 쓰든 그 위치부터 모든
  성부에 적용되고 블록 뒤에도 이어집니다. 같은 위치에서 두 성부가 다르게 정하면
  Warning이고, 첫 성부의 것을 씁니다.
- 블록 바로 뒤의 `{ }`는 또 하나의 성부입니다. 단, 뒤에 길이나 `~`가 오면
  화음입니다.
- 성부 안에는 무엇이든 쓸 수 있습니다: 화음, 함수, 반복.

#### Examples

```piano
voices{ E4 D C D E E E/2 }{ C3/1 G2/1 }
```

```piano
voices{ | C5 B4 A G | F/2 E/2 | }{ | C3/1 | F3/2 G/2 | }
```

```piano
C4/8 voices{ G4 E G E G E G E }{ p C3/1 } D   // D: E4 다음의 8분음표, mf
```

```piano
voices{ E4/2 }{ C4/2 } {C4 E G}/2   // 마지막 { }는 길이가 있으니 화음
```

```piano
| time 2/4 voices{ G4/4 A~ | time 3/4 A/4 B C5 }{ C3/2 | E3/2. } |
// 박자가 바뀌는 곳에서 오른손 A가 이어짐: 3/4는 두 성부 모두에 적용
```

```piano warning
voices{ C4 D E F }{ C3/2 }   // Warning: 4박 대 2박
```

#### See also

`guide hands` · `guide chord` · `guide pedal`

### Writing for two hands {#hands}

#### Concept

피아노에서 보통 오른손은 가운데 도 위에서 멜로디를, 왼손은 그 아래에서 베이스와
화음을 칩니다. `voices`에서는 손 하나가 성부 하나(또는 그 이상)입니다.

#### Patterns

- 화음을 누른 채 멜로디: 왼손이 마디마다 화음 하나를 누르고 있음.
- 펼친 화음 위의 멜로디: 왼손이 화음의 음을 8분음표로 하나씩 침.
- 베이스와 화음: 박에 낮은 음, 그 뒤에 화음.
- 옥타브 멜로디: 같은 선율을 한 옥타브 차이로 두 번 (`guide fn.up`).

#### Tips

- `voices` 블록 하나를 한두 마디로 하고, 한 줄에 하나씩 쓰세요. 그러면 `play 3`이
  그 부분을 양손으로 연주하고, 알림도 정확한 위치를 가리킵니다.
- 한 블록의 성부들은 길이를 같게 하세요. 안에 마디줄을 쓰면 검사에 도움이 됩니다.
- 각 성부의 시작에 옥타브를 쓰세요. 모든 성부는 다른 성부가 끝난 곳이 아니라
  블록 직전 상태에서 시작합니다.

#### Examples

```piano
// 양손 반짝반짝 작은 별, 한 줄에 한 마디
voices{ C4 C G4 G }{ C3/2 E/2 }
voices{ A4 A G/2 }{ F3/2 E/2 }
voices{ F4/4 F E E }{ D3/2 C/2 }
voices{ D4 D C/2 }{ G2/2 C3/2 }
```

```piano
voices{ E5/2 D/2 }{ C3/8 E G E C E G E }   // 펼친 화음 왼손
```

```piano
voices{ G4/1 }{ C3 {E3 G}/4 G2 {E3 G}/4 }   // 베이스와 화음
```

#### See also

`guide voices` · `guide fn.louder` · `guide pedal`

## 5. Dynamics and expression {#expression}

셈여림은 음악이 얼마나 센지 여린지입니다. 이 장에서는 ppp부터 fff까지의 기호,
점점 세게/여리게, 악센트, 구간 전체의 세기 바꾸기를 다룹니다.

### Dynamic marks {#dynamic}

#### Concept

기호는 이탈리아어에서 왔습니다: p(피아노)는 여리게, f(포르테)는 세게, m(메조)은
조금입니다. 기호는 건반을 얼마나 세게 치는지, 곧 벨로시티(1–127)를 정합니다.
세게 치면 소리가 커질 뿐 아니라 피아노 소리가 더 밝아집니다.

- `ppp` 매우매우 여리게: 16
- `pp` 매우 여리게: 33
- `p` 여리게: 49
- `mp` 조금 여리게: 64
- `mf` 조금 세게: 80 (기본값)
- `f` 세게: 96
- `ff` 매우 세게: 112
- `fff` 매우매우 세게: 127

#### Rules

- 기호는 다음 기호가 나올 때까지 뒤의 모든 음에 적용됩니다.
- 기호는 `f`를 빼고 대소문자를 가리지 않습니다: 소문자 `f` 하나는 포르테, `F`는
  음입니다.
- `voices` 안에서는 성부마다 기호를 따로 가집니다.

#### Examples

```piano
p C4 D E F f G A B C   // 여리게, 그다음 세게
```

```piano
f C4 D E/2 p C D E/2   // 한 구절과 그 메아리
```

```piano
mf C4 E G pp C5/2   // 갑자기 여린 맨 윗음
```

#### Common mistakes

- 기호가 계속 이어진다는 것을 잊기: `ff` 뒤로는 다음 기호까지 모든 음이 매우 세게
  납니다.

#### See also

`guide cresc` · `guide fn.louder` · `guide case`

### Crescendo and diminuendo {#cresc}

#### Concept

`cresc`(크레셴도)는 조금씩 세게, `dim`(디미누엔도)은 조금씩 여리게입니다. 둘 다
다음 세기 기호를 향해 갑니다.

#### Syntax

`cresc <음들> <기호>`, `dim <음들> <기호>`.

#### Rules

- 현재 세기에서 시작해 음마다 같은 간격으로 바뀌고(화음은 한 번으로 셈), 목표
  기호 다음 음에서 목표 세기에 도달합니다. `p cresc C D E F f G`는 49, 60, 72, 84,
  그리고 G가 96입니다.
- 뒤에 기호가 없으면 아무것도 바뀌지 않고 Warning이 납니다.
- `cresc`인데 더 여린 기호로 가면(`dim`은 반대) Warning이 납니다. 음은 그래도
  목표로 바뀝니다.
- `voices` 블록 밖의 cresc는 블록 안의 음을 세지 않습니다. 성부마다 따로 cresc를
  쓸 수 있습니다.

#### Examples

```piano
p cresc C4 D E F G A B f C5   // p에서 f까지 여덟 단계
```

```piano
ff dim C5 B A G F E D pp C   // 음계를 내려가며 사라짐
```

```piano
mp cresc | C4 D E F |
G A B C | ff D/1 |   // 두 줄에 걸친 크레셴도
```

```piano warning
cresc C4 D E   // Warning: 목표 기호가 없음
```

```piano warning
f cresc C4 D E p F   // Warning: 더 여린 기호를 향한 cresc
```

#### See also

`guide dynamic` · `guide fn.accent`

### Accents {#fn.accent}

#### Concept

악센트는 특정 음을 두드러지게 합니다: 마디마다 첫 박, 또는 동기 안에서 어떤 이름의
음 전부.

#### Syntax

`accent(1/<n>){ ... }` 또는 `accent(<음>){ ... }`. 해당하는 음의 벨로시티가
+20 됩니다 (최대 127).

#### Rules

- `1/n`: 첫 음부터 n번째 타건마다. 타건은 음이 시작되는 순간이고, 화음은 한 번으로
  세며 쉼표는 세지 않습니다. n은 1–64.
- `<음>`: 그 이름의 모든 음, 옥타브는 상관없음. 샵과 플랫은 소리로 셉니다:
  `accent(F#)`은 `accent(Gb)`와 같습니다.

#### Examples

```piano
accent(1/2){ C4/8 D E F G A B C }   // 매 박의 시작
```

```piano
time 3/4 accent(1/3){ C4 E G C5 G4 E }   // 왈츠: 마디마다 첫 박
```

```piano
accent(G){ G4 E G D G C }   // G가 모두 두드러짐
```

#### See also

`guide fn.louder` · `guide dynamic`

### Louder and softer {#fn.louder}

#### Concept

`louder`와 `softer`는 기호를 다시 쓰지 않고 구간 전체의 세기를 옮깁니다. 가장 흔한
쓰임은 균형입니다: 멜로디를 반주보다 조금 세게.

#### Syntax

`louder(<n>){ ... }`, `softer(<n>){ ... }`: 모든 음의 벨로시티에 n을 더하거나
뺍니다 (1–127 안으로 제한).

#### Rules

- 안의 기호도 그대로 동작하고, 그 위에 변화가 더해집니다.
- 정수라면 무엇이든 됩니다.

#### Examples

```piano
louder(20){ C4 D E F }   // mf(80)가 100이 됨
```

```piano
voices{ louder(15){ E5 D C D } }{ softer(15){ C3/8 E G E C E G E } }
```

#### See also

`guide dynamic` · `guide hands`

## 6. Pedals {#pedals}

피아노에는 페달이 세 개 있습니다. piano에서는 각 페달을 단어로 켜고 끄며, 페달끼리
자유롭게 겹쳐도 됩니다.

### Damper pedal {#pedal}

#### Concept

오른쪽 페달(댐퍼, 서스테인)은 모든 댐퍼를 들어 올립니다. 음이 길이가 끝난 뒤에도
계속 울리고 서로 섞입니다. 음악을 매끄럽고 풍성하게 하며, 화성이 바뀔 때 페달을
바꿔 주면 소리가 탁해지지 않습니다.

#### Syntax

`ped on`, `ped off`. 나머지 두 페달은 `sos on|off`(`guide sostenuto`)와
`soft on|off`(`guide soft-pedal`)입니다.

#### Rules

- 페달은 구간이 아니라 스위치입니다. 다시 바꿀 때까지 줄을 넘어서도 그대로
  유지됩니다.
- 쓴 순간에 동작합니다. 음 바로 앞에 쓴 페달은 그 음을 잡고, 음 바로 뒤에 쓰면
  그 음에는 늦습니다.
- 새 화음 바로 앞의 `ped off ped on`은 깔끔한 페달 바꾸기입니다: 앞의 소리는 멈추고
  새 화음은 잡힙니다.
- 이미 켜진 페달을 또 켜면 Hint가 납니다.
- 페달은 피아노 전체의 것입니다: `voices`에서 한 성부의 페달이 모든 성부에
  적용됩니다.
- 곡(song)에서는 조각 끝에 켜져 있던 페달이 그 조각 끝에서 떼어집니다.

#### Examples

```piano
ped on C4/8 E G C5 ped off   // 펼친 화음이 함께 울림
```

```piano
ped on {C3 G3}/2 {E4 G C5}/2 ped off ped on {F3 C4}/2 {F4 A C5}/2 ped off
```

```piano hint
ped on C4 ped on D   // Hint: 이미 켜져 있음
```

#### Common mistakes

- `ped off`를 잊기: 그 뒤의 모든 음이 다음 음까지 울려 음악이 뭉개집니다.

#### See also

`guide sostenuto` · `guide soft-pedal` · `guide connector`

### Sostenuto pedal {#sostenuto}

#### Concept

가운데 페달은 밟는 순간 울리고 있던 음만 계속 울리게 합니다. 그 뒤에 친 음은 평소처럼
끊깁니다. 낮은 베이스 음을 계속 울리면서 그 위의 멜로디는 깨끗하게 둘 수 있습니다.

#### Syntax

`sos on`, `sos off`.

#### Rules

- 잡을 음 바로 앞에 `sos on`을 쓰세요. 음이 시작되는 순간에 잡힙니다.
- 잡힌 음은 `sos off`까지 울립니다.

#### Examples

```piano
sos on C2/8 E4 F G A B C5 D/2 sos off   // C2가 깨끗한 멜로디 아래에서 울림
```

```piano
ped on C2/8 E4 F G A B C5 D/2 ped off   // 비교: 모든 음이 울림
```

#### See also

`guide pedal` · `guide soft-pedal`

### Soft pedal {#soft-pedal}

#### Concept

왼쪽 페달(우나 코르다)은 소리를 더 여리고 부드럽게 합니다. piano에서는 페달이 켜진
동안 친 음의 벨로시티를 80 %로 낮춥니다.

#### Syntax

`soft on`, `soft off`.

#### Examples

```piano
soft on p C4 E G C5/2 soft off   // 아주 조용하고 부드럽게
```

#### See also

`guide pedal` · `guide dynamic`

## 7. Repeats {#repeats}

음악은 같은 부분을 자주 되풀이합니다. 도돌이표, 다른 끝맺음, `%`, `seq`를 쓰면 같은
음을 두 번 쓰지 않아도 됩니다.

### Repeat signs {#repeat}

#### Concept

`|: ... :|`는 기호 사이를 두 번 연주합니다. 반복문과 같습니다. `:|` 뒤의 `x <n>`으로
횟수를 정합니다.

#### Syntax

`|: ... :|`, `|: ... :| x3` (`x 3`도 됨).

#### Rules

- 횟수는 1–99입니다. 그 밖은 Warning이고 2로 처리합니다.
- 매번 같은 소리가 납니다: 글자를 한 번만 읽으므로 옥타브가 밀리지 않습니다.
- 반복 뒤에는 글자 끝의 상태에서 이어집니다.
- `|:` 없이 `:|`만 쓰면 처음부터(또는 앞 반복이 끝난 곳부터) 반복하고 Hint가
  납니다.
- 반복 안에 반복을 넣을 수 있습니다.
- 반복 바로 뒤의 `%`는 복사할 마디가 없습니다.

#### Examples

```piano
|: C4 D E C :|   // 두 번 연주
```

```piano
|: C4/8 E G E :| x4   // 네 번
```

```piano
|: |: C4 D :| E F :|   // (C D C D E F)를 두 번
```

```piano hint
C4 D E F :|   // Hint: |: 없음, 처음부터 반복
```

```piano error
|: C4 D E   // Error: :| 없는 |:
```

#### See also

`guide volta` · `guide bar-repeat` · `guide fn.seq`

### Different endings {#volta}

#### Concept

반복하는 부분이 두 번째에는 다르게 끝나는 경우가 많습니다: 1번, 2번 끝맺음(볼타
괄호). 공통 부분은 한 번만 씁니다.

#### Syntax

`|: <공통> [1 <끝맺음 1> :| [2 <끝맺음 2> |`

#### Rules

- 끝맺음 개수만큼 연주합니다. k번째에는 공통 부분, 그다음 k번 끝맺음을 연주합니다.
- 끝맺음은 `[1`부터 차례로 번호를 붙입니다 (아니면 Warning).
- 마지막을 뺀 모든 끝맺음은 `:|`로 끝납니다. 마지막 끝맺음은 다음 `|`, 다음 `|:`,
  또는 조각의 끝에서 끝납니다.
- 모든 끝맺음은 공통 부분 뒤의 상태에서 시작합니다.

#### Examples

```piano
|: C4 D E F [1 G/1 :| [2 C/1 |   // C D E F G, 그다음 C D E F C
```

```piano
|: E4 D C D [1 E E E/2 :| [2 D D C/2 |
```

```piano
|: C4 E G/2 [1 C5/1 :| [2 E/1 :| [3 C4/1 |   // 끝맺음 셋
```

```piano error
C4 D [1 E   // Error: [1은 |: ... :| 안에서만
```

#### See also

`guide repeat`

### Repeat the previous bar {#bar-repeat}

#### Concept

`%`는 악보의 반복 기호(시밀레)처럼 "앞 마디와 같게"라는 뜻입니다. 되풀이되는 반주
마디를 계속 다시 쓰지 않아도 됩니다.

#### Rules

- `%`는 바로 앞 마디, 곧 마지막 두 `|` 사이의 음과 페달을 복사합니다.
- 그래서 앞에 끝난 마디가 있어야 합니다. 없으면 Warning입니다.
- `%` 뒤에 `%`를 써도 됩니다: 복사한 것을 다시 복사합니다.

#### Examples

```piano
| C3/8 E G E C E G E | % | % |   // 한 마디를 세 번 연주
```

```piano
| C4 E G E | % | F A C5 A4 | % |
```

```piano warning
C4 D % E   // Warning: 반복할 끝난 마디가 없음
```

#### See also

`guide bar` · `guide repeat`

### Sequences {#fn.seq}

#### Concept

반복진행(시퀀스)은 짧은 동기를 매번 조금씩 높이거나 낮춰 되풀이합니다. `seq`로 한
번에 씁니다.

#### Syntax

`seq(<횟수>, <간격>){ ... }`: 횟수 1–64, 간격은 반음 단위 (음수면 내려감).

#### Rules

- k번째는 k × 간격만큼 옮깁니다. 첫 번째는 그대로입니다.
- 간격은 음계의 한 칸이 아니라 반음이므로, 동기의 모양이 정확히 유지됩니다.
- A0–C8 밖으로 나간 음은 접히고 Warning이 납니다.

#### Examples

```piano
seq(3, 2){ C4/8 D E C }   // C D E C, D E F# D, E F# G# E
```

```piano
seq(4, -1){ G4 F E/2 }   // 반음씩 미끄러져 내려감
```

```piano
seq(2, 12){ C4 E G E }   // 같은 동기를 한 옥타브 위에서 다시
```

#### See also

`guide repeat` · `guide fn.up`

## 8. Functions {#functions}

함수는 구간 전체를 한 번에 바꿉니다: 올리거나 내리고, 세게 하거나 여리게 하고,
시간을 줄이거나, 옮기면서 반복합니다.

### How functions work {#fn.syntax}

#### Concept

함수는 구간에 적용하는 변환입니다: `이름(인자){ 본문 }`. 코드로 생각하면 본문의 음들에
대한 `map`과 같습니다. 내장 함수만 있습니다: `up`, `down`, `accent`, `louder`,
`softer`, `tuplet`, `seq`. 사용자 정의 함수와 재귀는 없습니다.

#### Rules

- 본문은 평범한 코드입니다: 화음, 반복, `voices`, 페달, 다른 함수.
- 중첩된 함수는 안쪽부터 바깥쪽으로 적용됩니다. 32단계까지 중첩할 수 있습니다.
- 본문은 변환 전에 쓴 그대로 읽으므로, 함수 뒤의 음은 마지막으로 쓴 음에서
  이어집니다.
- 결과가 A0–C8 밖이면 옥타브 단위로 접고 Warning을 냅니다.
- 모르는 이름이나 잘못된 인자는 Error입니다. 본문은 바뀌지 않은 채 연주됩니다.
- 붙임줄은 함수 안팎으로 이어집니다: `G~ tuplet(3){ G E C }`는 G를 이어서 끕니다.

#### Arguments

- `up(n)`, `down(n)`: 반음 수, 0–88.
- `accent(1/n)`: n은 1–64; 또는 `accent(<음>)`.
- `louder(n)`, `softer(n)`: 정수 무엇이든.
- `tuplet(n)`: 3–64.
- `seq(times, step)`: times는 1–64, step은 무엇이든.

#### Examples

```piano
up(12){ accent(1/3){ C4/8 D E F G A } }   // accent 먼저, 그다음 한 옥타브 위로
```

```piano
louder(10){ |: C4 E G :| }   // 함수 안의 반복
```

```piano error
up(100){ C4 }   // Error: 최대 88, C4는 그대로 연주됨
```

```piano error
echo(2){ C4 D }   // Error: 그런 함수는 없음
```

#### See also

`guide fn.up` · `guide fn.accent` · `guide fn.tuplet` · `guide fn.seq`

### Up {#fn.up}

#### Concept

`up(n)`은 구간의 모든 음을 반음 n개만큼 올립니다. 구절을 더 높게 다시 연주하거나,
구간의 조를 바꾸거나, 멜로디를 한 옥타브 위로 겹칠 때 씁니다.

#### Common steps

- 1: 반음. 2: 온음.
- 3: 단3도. 4: 장3도.
- 5: 완전4도. 7: 완전5도.
- 12: 옥타브.

#### Examples

```piano
C4 E G up(12){ C4 E G }   // 같은 것을 한 옥타브 위에서
```

```piano
up(2){ C4 D E F G/2 }   // 라장조로: D E F# G A
```

```piano
voices{ C4 D E F }{ up(12){ C4 D E F } }   // 옥타브로 겹친 멜로디
```

```piano warning
up(12){ C8 }   // Warning: C8보다 높음, 다시 접힘
```

#### See also

`guide fn.down` · `guide fn.seq` · `guide transform`

### Down {#fn.down}

#### Concept

`down(n)`은 구간의 모든 음을 반음 n개만큼 내립니다. `up`의 반대입니다.

#### Examples

```piano
down(12){ C4 E G }   // 한 옥타브 아래
```

```piano
voices{ C4/2 E/2 }{ down(12){ C4/2 E/2 } }   // 멜로디를 겹치는 베이스
```

```piano
G4 A B down(7){ G4 A B }   // 같은 동기를 완전5도 아래에서
```

#### See also

`guide fn.up` · `guide fn.syntax`

## 9. Song {#song}

곡(song)은 프로젝트의 조각들을 순서대로 늘어놓습니다. `song.piano`에 쓰고,
`play song`으로 연주하고, `song`으로 보고, 조각처럼 내보냅니다.

### Arrangement {#arrangement}

#### Concept

조각은 함수, 곡은 `main()`이라고 생각하세요. 각 부분은 한 번만 쓰고 필요한 만큼
불러냅니다.

#### Syntax

`<조각> > <조각> > ...`: `p1 > bridge1 > p4`.

#### Rules

- 이름은 `.piano`를 뺀 조각 파일 이름입니다. 곡이 자기 자신을 넣을 수는 없습니다.
- 조각과 마찬가지로 줄바꿈은 자유이고 `//` 주석을 쓸 수 있습니다.
- 각 조각은 독립적입니다: 프로젝트 기본값에서 시작해 자기 템포, 세기, 옥타브
  기준음을 가지며, 켜 둔 페달은 조각 끝에서 떼어집니다.
- 같은 조각을 몇 번이든 쓸 수 있습니다.
- 없는 조각은 Error이고, 나머지는 연주됩니다. 조각 안의 알림은 곡의 해당 줄에
  `[p2 line 3] ...`처럼 나타납니다.
- 곡 파일은 `song.piano`이거나, `piano.toml`에서 정한 `entry`입니다.

#### Examples

```song
p1 > p2 > p1
```

```song
// 인트로, 1절 두 번, 후렴, 1절, 마무리
intro > verse > verse > chorus
verse > outro
```

#### See also

`guide connector` · `guide transform` · `guide file`

### Connectors {#connector}

#### Concept

조각 사이를 어떻게 이을지 정할 수 있습니다: 바로, 서서히 줄이며, 쉬었다가, 또는
이음새에서 페달을 밟은 채로.

#### Syntax

- `>`: 바로 이어서.
- `fade`: 앞 조각의 끝을 서서히 줄임.
- `r/<n>`: 조각 사이의 쉼.
- `ped`: 이음새에서 댐퍼 페달을 밟고 있음.

#### Rules

- `fade`는 바로 앞 조각의 마지막 4박에 있는 음을 점점 더 여리게 해서, 맨 끝에서
  벨로시티의 30 % 가까이까지 낮춥니다.
- `r/<n>`은 조각 안의 쉼표처럼 길이와 점을 씁니다. 그냥 `r`은 한 박입니다.
- `ped`는 앞 조각이 끝나기 한 박 전에 댐퍼를 밟고, 다음 조각이 시작되고 한 박 뒤에
  뗍니다. 그래서 이음새가 울립니다.
- `fade`와 `ped`는 바로 앞에 조각이 있어야 합니다 (사이에 쉼이 있으면 Warning).
- 두 조각을 잇기 위해 만든 짧은 연결 조각(브리지)도 그냥 일반 조각입니다.

#### Examples

```song
p1 > fade > r/2 > p2
```

```song
p1 > ped > p2 > ped > p3
```

```song warning
fade > p1   // Warning: fade 앞에 조각이 필요함
```

#### See also

`guide arrangement` · `guide pedal`

### Transforming pieces {#transform}

#### Concept

내장 함수는 곡 안의 조각에도 쓸 수 있습니다: 마지막 후렴을 온음 높게, 반복을 더
여리게. 조각 파일은 바뀌지 않습니다.

#### Syntax

`<함수>(<인자>){ <조각들> }`.

#### Examples

```song
chorus > up(2){ chorus }   // 마지막 후렴을 온음 높게
```

```song
p1 > softer(20){ p1 } > louder(10){ p2 > p3 }
```

```song
seq(3, 2){ p1 }   // p1을 세 번, 매번 온음씩 높게
```

```song error
up(100){ p1 }   // Error: up은 0-88
```

#### See also

`guide fn.syntax` · `guide arrangement`

## 10. Alerts {#alerts}

알림은 편집기가 문제를 알려 주는 방법입니다. 알림이 음악을 멈추지는 않습니다.
연주할 수 있는 것은 연주합니다. 이 장에서는 알림의 원인과 고치는 법을 정리합니다.

### Alert levels {#levels}

#### Concept

- ✖ Error: 그 부분을 연주할 수 없어 건너뜁니다 (모르는 단어, 잘못된 길이).
- ⚠ Warning: 연주는 되지만 의도와 다를 수 있습니다 (옥타브 접기, 애매한 도약,
  길이가 틀린 마디, 무시된 붙임줄).
- · Hint: 참고 정보입니다 (`|:` 없는 `:|`, 이미 켜진 페달).

#### Where they show

- Output 화면에 줄 번호와 함께 나옵니다 (`output`).
- Code의 줄 번호 옆에 그 줄에서 가장 심각한 알림 기호가 표시됩니다.
- 입력하는 동안 바로바로 갱신됩니다.

#### Examples

```piano warning
C4 F#   // Warning: line 1: C4 → F# is ambiguous (6 semitones)
```

#### See also

`guide errors` · `guide warnings` · `guide hints`

### Errors {#errors}

#### Notes and words

- unknown word: 오타, 계이름(`do`), 빠진 공백(`CDE`), 독일식 `H`, 대문자 플랫
  (`EB`). 철자를 확인하세요.
- unknown symbol: 언어에 없는 문자. 예: `$`, 혼자 쓴 `#`, 길이 없는 점(`C.`).
- unexpected number: 음과 옥타브 사이의 공백(`C 4`). `C4`로 쓰세요.
- length without a note: 앞에 음이 없는 `/8`.
- invalid length: `/3`, `/6`, ... 1 2 4 8 16 32 64 128만 됩니다. 셋잇단음표는
  `tuplet`으로 씁니다.

#### Headers and pedals

- tempo must be 20-400, `tempo` needs a number: `tempo 90`처럼 쓰세요.
- invalid time signature, `time` needs a signature: `time 3/4`처럼 쓰세요.
- `ped` needs `on` or `off` (`sos`, `soft`도 마찬가지).

#### Brackets

- only notes are allowed in a chord; chord `{` has no matching `}`; unmatched `}`.
- no bar lines inside `free`: `free` 블록은 한 마디입니다; `free{` has no matching `}`.
- `|:` has no matching `:|`; `[2` outside a repeat; `x` needs a number.

#### Functions

- unknown function: `up down accent louder softer tuplet seq`만 있습니다.
- invalid arguments: 허용 범위는 `guide fn.syntax`를 보세요. 메시지에 사용법이
  함께 나옵니다.
- needs a body `{ ... }`; `(` has no matching `)`; `{` has no matching `}`.

#### Songs

- piece not found: 프로젝트에 그런 `.piano` 파일이 없습니다.
- a song cannot contain itself; unexpected ... in song; missing `}` in song.
- `>` is only used in song files: 조각 안에 `>`를 썼습니다.

#### Examples

```piano error
C 4 D   // Error: 예상치 못한 숫자, C4로 쓸 것
```

#### See also

`guide levels` · `guide warnings`

### Warnings {#warnings}

#### Pitch

- ambiguous (6 semitones): 어느 쪽으로든 반음 6개인 도약. 옥타브 숫자를 쓰세요.
- folded to ...: A0–C8 밖의 음이라 옥타브 단위로 옮겼습니다.

#### Length and rhythm

- at most 2 dots, using 2.
- bar has N beats, 4/4 expects 4: 마디 길이가 박자표와 맞지 않습니다
  (`guide bar`).
- tie is not followed by the same note, ignored; `~` must follow a note or chord;
  rests cannot be tied.
- `%` has no previous bar to repeat.

#### Chords and voices

- put the chord length after `}`; empty chord.
- voices have different lengths (가장 긴 성부를 씀).
- voices set `tempo` (or `time`) differently at the same point: 첫 성부의 것을
  씁니다.

#### Repeats and dynamics

- repeat count must be 1-99, using 2; repeat count without a repeat; 첫 끝맺음이
  `[1`이 아님.
- cresc/dim has no target dynamic; cresc가 더 세지 않은 기호에서 끝남 (dim은 더
  여리지 않은 기호).
- 함수나 블록이 32단계보다 깊게 중첩됨.

#### Songs

- `fade` / `ped` needs a piece before it.

#### Examples

```piano warning
| C4 D E | F G A B |   // Warning: 마디가 3박, 4/4는 4박
```

#### See also

`guide levels` · `guide hints`

### Hints {#hints}

#### List

- `:|` without `|:` repeats from the start: 의도한 것이라면 괜찮습니다.
- `ped on` while already on (또는 꺼진 상태에서 off): 페달 상태가 바뀌지 않았습니다.
- `voices` has one voice: `{ ... }`를 하나 더 쓰세요.
- `~` after the chord ties every note: 화음 안의 `~`는 필요 없습니다.

#### Examples

```piano hint
voices{ C4 D E F }   // Hint: 성부가 하나뿐
```

#### See also

`guide levels`

## 11. Reference {#reference}

언어 전체를 한 쪽에, 기본값과 바꾸는 법, 그리고 이 설명서에 나오는 음악 용어를
모았습니다.

### Grammar at a glance {#grammar}

#### Pieces

```text
note      C D E F G A B  [# b]  [0-8]  [/n [. ..]]  [~]
rest      r [/n [. ..]]
chord     { note [~] ... } [/n [. ..]] [~]
comment   // ...
header    tempo <20-400>   time <beats>/<unit>
bar       |   %
dynamic   ppp pp p mp mf f ff fff   cresc   dim
pedal     ped|sos|soft on|off
repeat    |: ... :| [x n]   |: ... [1 ... :| [2 ... |
voices    voices{ ... }{ ... }
free      free{ ... }
function  up down accent louder softer tuplet seq   name(args){ ... }
```

#### Songs

```text
song      piece > piece   fade   r/n   ped   name(args){ pieces }
```

#### A piece using most of it

```piano
tempo 90 time 3/4
| p G4/8 A B C5 D E | ped on {C4 E G}/2. ped off |
|: voices{ E5/4 D C }{ C3/2. } :| x2
cresc | up(12){ C4 D E } | f G5/2. |
```

#### See also

`guide first` · `man grammar`

### Defaults and settings {#defaults}

#### Builtin defaults

tempo 100, time 4/4, 길이 `/4`, 세기 `mf`, 옥타브 기준음 C4.

#### Changing them

프로젝트 하나에는 `piano.toml`, 모든 프로젝트에는 `~/.config/piano/config.toml`의
`[defaults]`에 씁니다. 모든 항목은 선택 사항입니다.

```toml
[defaults]
tempo = 90
time = "3/4"
length = "1/8"
dynamic = "mp"
octave = 3        # 시작 기준음이 C3
```

#### Order

조각 안에 쓴 것이 가장 우선이고, 그다음 `piano.toml`, 그다음 전역 설정, 마지막으로
내장 기본값입니다. 곡의 모든 조각은 이 기본값에서 시작합니다.

#### Examples

```piano
tempo 120 C D E   // 조각에 쓴 템포가 기본값보다 우선
```

#### See also

`guide continuity` · `guide tempo`

### Music terms {#glossary}

#### Terms

- accent (악센트): 이웃한 음보다 세게 치는 음.
- arpeggio, broken chord (펼침화음): 화음의 음을 하나씩 차례로 치는 것.
- bar, measure (마디): 박의 묶음, `|`로 끝남.
- beat (박): 음악 시간의 단위. piano에서는 4분음표.
- cadenza (카덴차): 대개 작은 음표의 흐름으로, 엄격한 박 없이 자유롭게 연주하는
  구간. `free`로 씀.
- chord (화음): 함께 치는 여러 건반.
- crescendo / diminuendo (크레셴도 / 디미누엔도): 점점 세게 / 점점 여리게.
- dynamics (셈여림): 소리의 세기, ppp부터 fff까지.
- octave (옥타브): 반음 12개. 이름이 같은 음의 위아래.
- pickup, anacrusis (못갖춘마디, 여린내기): 첫 온전한 마디 앞의 음.
- semitone (반음): 한 건반에서 바로 옆 건반까지의 걸음.
- sequence (반복진행, 시퀀스): 높이를 옮겨 되풀이하는 동기.
- tempo (템포): 빠르기, 1분당 박 수.
- tie (붙임줄): 같은 높이의 두 음을 하나로 이은 것.
- time signature (박자표): 어떤 길이의 박 몇 개가 한 마디인지.
- transpose (조옮김): 음을 일정한 반음 수만큼 올리거나 내림.
- triplet, tuplet (셋잇단음표, 잇단음표): 둘(또는 그보다 적은) 음의 시간에 셋(또는 n개)을
  연주하는 것.
- velocity (벨로시티): 건반을 치는 세기, 1–127.
- voice (성부): 음악의 한 줄기. 여러 성부가 동시에 울림.
- volta (볼타, 1·2번 괄호): 반복의 첫 번째, 두 번째 끝맺음.

#### See also

`guide grammar`
