# piano Tutorial (Korean)

Same IDs and `Expect`, `Piece`, `Type`, `Line` lines as `docs/TUTORIAL.md`. Titles and
field names stay in English; descriptions and hints are Korean.

## WELCOME {#screen}

    이 튜토리얼에서는 곡 하나를 한 줄씩 직접 완성함:
    "반짝반짝 작은 별". 모든 줄을 Code 영역에 직접 입력하고,
    무언가를 적는 단계는 자동으로 확인해 줌 (Check ✔).

    화면은 네 영역으로 나뉨:
      Keys (위)        피아노 건반, 항상 보임
      Code (왼쪽)      작성 중인 조각, 줄 번호 포함
      Display (오른쪽) 이 튜토리얼, 결과, 도움말, 설명서 ...
      Cmd (아래)       play, save, help 같은 명령어
    F12로 영역을 옮기고 (반대 방향: Shift+F12), 바로 가려면
    F2 Keys, F3 Code, F4 Cmd. 지금 위치는 맨 아래 상태줄에 보임.

    영역을 옮겨도 Display는 그대로임. 명령으로만 바뀜:
    played, ref, output, song, help, man, tutorial.
    Shift+PgUp / Shift+PgDn으로 어느 영역에서든 스크롤함.

    튜토리얼 이동 (Cmd, F4에서):
      tutorial next    다음 단계
      tutorial prev    이전 단계
      tutorial 5       5단계로 이동
    Try      F12를 몇 번 누르고, F4를 눌러 tutorial next 입력하기.

## LISTEN {#listen}

    먼저 귀로 멜로디를 찾아봄. F2를 누르면 키보드가 피아노가 됨.
    Keys에서는 소리만 나고 아무것도 적히지 않음.

      아랫줄      Z X C V B N M  =  C D E F G A B  (도 레 미 파 솔 라 시)
      검은 건반   S D   G H J    =  C# D#   F# G# A#
      윗줄        Q W E R T Y U  =  같은 음, 한 옥타브 위
      ← →         건반 전체를 한 옥타브씩 옮김

    작은 별은 두 소절로 시작함. 쳐 보기:
      Z Z B B N N B      C C G G A A G      도 도 솔 솔 라 라 솔
      V V C C X X Z      F F E E D D C      파 파 미 미 레 레 도
    길게 누르면 계속 울리고, 톡 치면 짧게 울림.
    Try      F2를 누르고 두 소절을 쳐 보기.
    Hint     Cmd `played`로 친 음 목록을 봄; `tutorial`로 돌아옴.

## FIRST LINE {#first}

    이제 첫 소절을 적음. F3(Code)을 누름. 조각 p1에는 `// p1`
    한 줄이 있음: //부터 그 줄 끝까지는 주석이라 무시됨.
    그래서 End를 누른 뒤 Enter를 누르고, 2번 줄에 입력:

    Type     | C4 C G G | A A G/2 |

    각 부분의 뜻:
      C4      4옥타브의 C(도), 피아노 한가운데 도
      C G G   숫자가 없으면 바로 앞 음과 가장 가까운 옥타브
      /2      2분음표(2박). /가 없는 음은 4분음표(1박).
      |       마디줄. 없어도 되지만 읽기 쉽고, 길이가 틀린
              마디는 줄 번호 옆에 ⚠가 표시됨.
    띄어쓰기 개수는 상관없고, c g a도 C G A와 같음.
    Expect   C4 C G G A A G/2
    Hint     man octave, man length, man bar.

## SECOND LINE {#second}

    Enter를 누르고 3번 줄에 둘째 소절을 적음:

    Type     | F/4 F E E | D D C/2 |

    왜 /4를 다시 적을까? 길이는 이어받음: 각 음은 바로 앞 음의
    길이를 그대로 씀. 2번 줄이 G/2로 끝났으니, /4가 없으면 이 F도
    2분음표가 됨.
    옥타브도 같은 방식으로 이어받음: F는 앞의 G 옆(F4)에,
    마지막 C는 앞의 D 옆(C4)에 놓임.
    Expect   F4/4 F E E D D C/2
    Hint     man inheritance. Cmd `ref`로 커서 위치의 상태를 봄.

## PLAY {#play}

    적은 것을 들어 봄. F4를 누르고 명령을 입력:
      play           조각 전체
      play 3         3번 줄만
      loop 2-3       2~3번 줄을 계속 반복
      stop           정지 (F6도 됨)
    F5는 Code 커서 위치부터 재생함: F3을 누르고 커서를 2번 줄로
    올린 뒤(↑, Home) F5를 누름.
    Try      F4, play, Enter. 그다음 F3, ↑, Home, F5.
    Hint     소리가 안 나면 Cmd `output`으로 이유를 확인 (font로 사운드폰트 지정).

## TEMPO {#tempo}

    tempo는 빠르기: 1분에 4분음표 몇 개 (기본 100).
    코드와 주석은 한 줄에 같이 쓸 수 있음. 1번 줄로 가서(F3, ↑)
    이렇게 바꿈:

    Line 1   tempo 90 // p1

    time 3/4라고 쓰면 한 마디가 3박이 됨 (기본 4/4).
    Try      다시 재생해 보기: 조금 느려짐.
    Hint     man tempo, man time.

## DYNAMICS {#dynamics}

    여리게 시작해서 점점 세게. 2번과 3번 줄을 바꿈
    (↑ ↓ Home End로 이동, Backspace로 지움):

    Line 2   | p C4 C G G | A A G/2 |
    Line 3   | cresc F/4 F E E | D D C/2 mf |

    셈여림 기호, 아주 여리게부터 아주 세게까지:
      ppp pp p mp mf f ff fff     (p = 여리게, mf = 보통, f = 세게)
    기호는 다음 기호가 나올 때까지 유지됨. cresc는 다음 기호(여기서는
    mf)까지 한 음씩 점점 세짐; dim은 반대로 점점 여려짐.
    Try      재생해서 3번 줄이 점점 커지는지 들어 보기.
    Hint     man dynamic, man cresc. Cmd `undo`로 수정을 되돌림.

## NEW PIECE AND REPEAT {#repeat}

    곡의 가운데는 새로운 부분이라 따로 조각(파일)을 만듦.
    Cmd(F4)에서 입력: new p2
    이제 Code에 빈 조각 p2가 보임. F3을 누르고 1번 줄에 입력:

    Piece    p2
    Type     |: G4 G F F | E E D/2 :|

    |: ... :|는 그 구간을 두 번 연주함; :| x3은 세 번.
    왜 G가 아니라 G4일까? 새 조각은 처음부터 시작함: 옥타브 기준 C4,
    길이 /4, mf. C4에서 가장 가까운 G는 아래쪽 G3이라서 G4로 적음.
    Expect   |: G4 G F F | E E D/2 :|
    Hint     man repeat, man volta. Cmd `song`으로 조각 목록을 봄.

## ENDING {#ending}

    쉼표와 큰 화음으로 끝냄. 세 번째 조각에 적음.
    Cmd에서: new end. 그다음 F3을 누르고 입력:

    Piece    end
    Type     r/2 ped on {C4 E G C}/1 ped off

      r/2      2분쉼표 (2박 동안 쉼)
      { }      같이 누르는 음: 화음. 길이는 } 뒤에 적음.
               화음 안에서는 각 음이 바로 앞 음과 가장 가까운
               옥타브가 되므로, 마지막 C는 C5.
      /1       온음표 (4박)
      ped on   댐퍼(오른쪽) 페달: ped off까지 음이 계속 울림.
               sos on|off (가운데), soft on|off (왼쪽)도 있음.
    Expect   {C4 E G C5}/1
    Hint     man chord, man rest, man pedal.

## SONG {#song}

    이제 부분들을 순서대로 이어 붙임. Cmd에서: open song.
    song 파일은 >로 이은 조각 목록임. 1번 줄을 이렇게 바꿈:

    Piece    song
    Line 1   p1 > p2 > p1 > end

    곡 전체 재생: F4, play song.
    조각 사이에는 fade(점점 사라짐), r/2(쉼), ped(이음새에서 페달
    유지)도 쓸 수 있음: p1 > fade > p2.
    Try      F4, play song.
    Hint     man arrangement, man connector.

## FUNCTIONS {#functions}

    함수는 다시 적지 않고 구간을 바꿈. 마지막 A 부분을 한 옥타브
    위로 연주해 봄: song의 1번 줄을 이렇게 바꿈

    Piece    song
    Line 1   p1 > p2 > up(12){ p1 } > end

    up(12)는 반음 12개(한 옥타브)만큼 올림. 다른 함수:
      down(n)  louder(n)  softer(n)  accent(1/3)  tuplet(3)  seq(3, 2)
    조각 안의 음에도 쓸 수 있음: up(12){ C4 E G }.
    Try      F4, play song.
    Hint     Cmd `ref`로 모든 함수를 봄. man fn.

## SAVE AND EXPORT {#save}

    save는 Code에 보이는 조각을 저장함. 저장하지 않은 작업은
    복구본에도 남음 (recover). 모든 조각을 저장:
      open p1, save      open p2, save
      open end, save     open song, save
    그다음 공유할 파일 만들기: export midi, export wav
    (사운드폰트 필요).
    Try      F4: 조각마다 save, 그다음 export midi.
    Hint     help export.

## MORE {#more}

    곡 하나를 완성함. 연습용 조각(new try)에서 한 줄씩 더 해 보기:
      C C# D Eb E               올림(#)과 내림(b)
      C/4. D/8 E/2              . 점음표 (1.5배 길이)
      G/2~ G/4                  ~ 붙임줄: 하나의 긴 음
      tuplet(3){ C/8 D E }      2개 길이에 3개 (셋잇단음표)
      |: C D [1 E :| [2 G |     반복할 때 끝을 다르게
      | C D E F | % |           %는 바로 앞 마디를 반복
      time 3/4 | C D E |        한 마디 3박
    설명서에 모든 규칙이 있음:
      man            장 목록           man octave     한 항목
      lang en        영어              tutorial 1     처음부터
    Try      F4, new try, 그다음 위의 줄을 입력해 보기.
