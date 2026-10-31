"""Explicit diagnostic knowledge: hypotheses and read-only checks.

Hand-written and inspectable. It is not learned and it is not a list of fixes:
every entry states what would be learned, never what to install.
"""

# id: statement, signals that raise it, symptoms that raise it, checks that discriminate it.
HYPOTHESES = {
    "library-version-mismatch": dict(
        statement="O componente que pede o símbolo foi feito para uma versão de biblioteca diferente da carregada.",
        signals=["undefined-symbol"], symptoms=[], checks=["identify-requiring-object", "resolve-library-path", "inspect-exported-symbols", "compare-runtime-host"]),
    "wrong-library-resolved": dict(
        statement="O carregador encontrou outra cópia da biblioteca (LD_LIBRARY_PATH, runtime da Steam ou pacote local) antes da esperada.",
        signals=["undefined-symbol", "missing-library"], symptoms=[], checks=["resolve-library-path", "review-launch-options", "compare-runtime-host"]),
    "launch-option-side-effect": dict(
        statement="Uma opção de inicialização ou variável fornecida desativa ou altera o componente que falha.",
        signals=["missing-library"], symptoms=[], needs_options=True, checks=["review-launch-options"]),
    "missing-windows-component": dict(
        statement="Falta no prefixo um componente Windows que o jogo importa.",
        signals=["missing-library"], symptoms=[], checks=["inspect-import-chain", "reset-prefix-with-backup", "verify-game-files"]),
    "wine-gap": dict(
        statement="O jogo chama algo que esta versão do Wine/Proton ainda não implementa.",
        signals=["unimplemented-function"], symptoms=[], checks=["test-other-proton", "collect-proton-log"]),
    "game-or-wine-fault": dict(
        statement="Há uma falha de acesso dentro do jogo ou de uma DLL; o módulo responsável ainda não é conhecido.",
        signals=["unhandled-exception"], symptoms=["crash"], checks=["locate-faulting-module", "collect-proton-log", "test-other-proton"]),
    "native-fault": dict(
        statement="Um processo nativo do Linux terminou com erro, fora do código Windows do jogo.",
        signals=["native-segfault"], symptoms=[], checks=["collect-native-backtrace", "check-driver-stack"]),
    "gpu-or-driver-fault": dict(
        statement="A GPU ou o driver perdeu o dispositivo durante a renderização.",
        signals=["device-lost"], symptoms=[], checks=["read-kernel-gpu-messages", "check-driver-stack", "isolate-scope"]),
    "graphics-translation-issue": dict(
        statement="A tradução gráfica (DXVK ou VKD3D-Proton) relata erro ou o driver não atende ao que ela pede.",
        signals=["graphics-api-error"], symptoms=["graphics", "black-screen"], checks=["check-vulkan-support", "check-driver-stack", "test-other-proton", "collect-proton-log"]),
    "prefix-state": dict(
        statement="O prefixo do jogo está em um estado inesperado.",
        signals=["prefix-error"], symptoms=[], checks=["inspect-prefix-error", "reset-prefix-with-backup"]),
    "anti-cheat-support": dict(
        statement="O anti-cheat do jogo não aceita este ambiente; pode não ser algo corrigível localmente.",
        signals=["anti-cheat"], symptoms=["online"], checks=["check-anticheat-support", "isolate-scope"]),
    "audio-path": dict(
        statement="A rota de áudio (servidor, dispositivo ou formato) falha ao iniciar.",
        signals=["audio-error"], symptoms=["audio"], checks=["inspect-audio-route", "collect-proton-log"]),
    "media-codec": dict(
        statement="O jogo depende de reprodução de mídia que o ambiente não decodifica.",
        signals=["media-error"], symptoms=["video"], checks=["inspect-media-errors", "test-other-proton"]),
    "runtime-container": dict(
        statement="O contêiner de runtime da Steam falhou ou está incompatível com o host.",
        signals=["runtime-container"], symptoms=[], checks=["compare-runtime-host", "collect-terminal-output"]),
    "thread-deadlock": dict(
        statement="O jogo ficou preso aguardando outra thread; a causa ainda precisa ser localizada.",
        signals=["sync-timeout"], symptoms=["freeze"], checks=["collect-proton-log", "locate-blocking-thread", "test-other-proton"]),
    "gpu-physx-chain": dict(
        statement="O PhysX do jogo não encontra CUDA dentro do Wine e cai para CPU.",
        signals=[], symptoms=["physx"], requires_vendor="nvidia", checks=["check-physx-chain", "measure-baseline"]),
    "physx-unavailable-hardware": dict(
        statement="PhysX por GPU depende de CUDA da NVIDIA; nesta GPU os efeitos rodam na CPU.",
        signals=[], symptoms=["physx"], excludes_vendor="nvidia", needs_vendor=True, checks=["measure-baseline", "reduce-physx-setting-check"]),
    "performance-configuration": dict(
        statement="O desempenho depende de configuração, cache de shaders ou da GPU em uso; ainda não há medição.",
        signals=[], symptoms=["performance", "stutter"], checks=["measure-baseline", "confirm-active-gpu", "check-driver-stack"]),
    "input-mapping": dict(
        statement="O dispositivo chega ao jogo por uma rota errada (Steam Input, SDL ou HID).",
        signals=[], symptoms=["input"], checks=["inspect-input-route", "collect-proton-log"]),
    "unknown-cause": dict(
        statement="A causa é desconhecida com a evidência atual.",
        signals=[], symptoms=[], always=True, checks=["collect-proton-log", "collect-system-info"]),
}

# what: the question. how: read-only instruction shown to a person; never executed here.
# plays_game: needs the game launched by the person. demo: matching maintainer demonstration class.
CHECKS = {
    "collect-proton-log": dict(what="Obter um log do Proton da falha.", how="Adicionar PROTON_LOG=1 %command% às opções do jogo, reproduzir uma vez, ler $HOME/steam-<AppID>.log. Só você altera isso.", cost=1, plays_game=True, demo="collect-proton-log", provides="log"),
    "collect-system-info": dict(what="Registrar o ambiente real: GPU, driver, kernel e Proton.", how="Steam → Ajuda → Diagnóstico do Steam Runtime; copiar o resultado.", cost=1, demo="collect-system-info", provides="info"),
    "collect-terminal-output": dict(what="Ver o que aparece antes do jogo iniciar.", how="Fechar a Steam, rodar `steam` em um terminal e ler a saída ao iniciar o jogo, ou ler ~/.local/share/Steam/logs/console_log.txt.", cost=2, plays_game=True, demo="collect-terminal-output"),
    "identify-requiring-object": dict(what="Descobrir qual arquivo exige o símbolo citado.", how="Ler a linha completa do erro e o contexto anterior no log: o caminho antes de `undefined symbol` nomeia quem pediu.", cost=1),
    "resolve-library-path": dict(what="Descobrir de onde a biblioteca foi carregada.", how="Executar `ldd <arquivo>` no objeto identificado; conferir LD_LIBRARY_PATH nas opções do jogo. Comparar com a cópia do runtime.", cost=1),
    "inspect-exported-symbols": dict(what="Verificar se a biblioteca carregada exporta o símbolo e qual a versão dela.", how="`nm -D --defined-only <biblioteca> | grep <símbolo>` e `readelf -d <biblioteca>`; conferir arquitetura com `file`.", cost=1),
    "compare-runtime-host": dict(what="Comparar a cópia do runtime da Steam com a do sistema.", how="Conferir a ferramenta e o runtime selecionados e a versão da mesma biblioteca em cada um. Trocar arquivos não faz parte desta verificação.", cost=2),
    "inspect-import-chain": dict(what="Ver qual executável importa a DLL ausente e se ela existe no prefixo.", how="Ler a linha `import_dll` e procurar a DLL em <prefixo>/drive_c/windows/system32 e na pasta do jogo.", cost=1),
    "locate-faulting-module": dict(what="Identificar o módulo do endereço da falha.", how="No log, ler o backtrace depois de `Unhandled exception` e o módulo que contém o endereço.", cost=1),
    "collect-native-backtrace": dict(what="Obter o backtrace do processo nativo que falhou.", how="`coredumpctl list` e `coredumpctl info <PID>`, somente leitura.", cost=2),
    "read-kernel-gpu-messages": dict(what="Procurar erros da GPU no kernel no momento da falha.", how="`journalctl -k -b | grep -iE 'xid|gpu hang|ring .* timeout'`.", cost=1),
    "check-vulkan-support": dict(what="Conferir se o Vulkan funciona e qual dispositivo responde.", how="`vulkaninfo --summary`; comparar com a GPU esperada e, em jogos 32 bits, com o driver 32 bits.", cost=1),
    "check-driver-stack": dict(what="Conferir a versão e a origem do driver.", how="Ler a versão no Diagnóstico do Steam Runtime; comparar com a citada nas fontes. Não implica atualizar.", cost=1, demo="check-driver-stack"),
    "inspect-prefix-error": dict(what="Ler o erro do prefixo e o caminho afetado.", how="Ler o traceback no log; listar o caminho citado em compatdata/<AppID>/pfx sem alterar nada.", cost=1),
    "reset-prefix-with-backup": dict(what="Testar se um prefixo novo muda o resultado.", how="Só com sua decisão: renomear compatdata/<AppID> preservando saves. Altera estado; o FRX não faz isso nesta pesquisa.", cost=4, changes_state=True, plays_game=True, demo="reset-prefix"),
    "test-other-proton": dict(what="Ver se o comportamento muda em outra versão do Proton.", how="Escolher Proton Experimental ou outra versão nas propriedades do jogo e reproduzir. Altera a configuração do jogo; só você decide.", cost=3, changes_state=True, plays_game=True, demo="test-other-proton"),
    "review-launch-options": dict(what="Revisar as opções de inicialização e testar sem elas.", how="Ler as opções atuais; comparar com o que o log mostra em `Options:`. Removê-las é decisão sua.", cost=1, demo="review-launch-options"),
    "verify-game-files": dict(what="Verificar a integridade dos arquivos do jogo.", how="Propriedades do jogo → Arquivos instalados → Verificar integridade.", cost=2, demo="verify-game-files"),
    "isolate-scope": dict(what="Saber se o problema é só neste jogo ou ambiente.", how="Outros jogos com a mesma API se comportam igual? Acontece em outra GPU ou no Windows?", cost=2, demo="isolate-scope"),
    "check-anticheat-support": dict(what="Confirmar se o anti-cheat aceita Linux neste jogo.", how="Consultar a página do jogo e o estado declarado pelo desenvolvedor; distinguir de suporte offline.", cost=1),
    "inspect-audio-route": dict(what="Conferir servidor, dispositivo e formato de áudio.", how="`pactl info`, `pactl list short sinks`; comparar com o erro do log.", cost=1),
    "inspect-media-errors": dict(what="Ler quais formatos de mídia falharam.", how="Procurar `mfplat`, `winegstreamer` ou `gstreamer` no log; anotar o formato pedido.", cost=1),
    "locate-blocking-thread": dict(what="Ver qual thread segura o bloqueio.", how="Ler a linha `blocked by` no log e localizar o que essa thread fazia.", cost=2),
    "check-physx-chain": dict(what="Verificar a cadeia de PhysX por GPU.", how="O jogo traz PhysX? A ferramenta fornece o nvcuda real? O nvcuda do prefixo é o stub? O driver tem libcuda da arquitetura do jogo?", cost=1),
    "measure-baseline": dict(what="Medir antes de concluir.", how="Capturar uma sessão no Performance Lab no mesmo trecho; sem medição não há regressão comprovada.", cost=3, plays_game=True),
    "confirm-active-gpu": dict(what="Confirmar qual GPU o jogo usa.", how="Ver o renderizador na captura ou em `MANGOHUD=1`. A GPU detectada no sistema não prova qual renderiza o jogo.", cost=2, plays_game=True),
    "reduce-physx-setting-check": dict(what="Ver como o jogo se comporta com PhysX reduzido nas opções.", how="Comparar o mesmo trecho com a opção do próprio jogo; registrar o resultado.", cost=2, plays_game=True),
    "inspect-input-route": dict(what="Ver por qual rota o dispositivo chega ao jogo.", how="Conferir Steam Input nas propriedades e o que `PROTON_LOG=+hid,+input` registra.", cost=2, plays_game=True),
}
FORBIDDEN_WORDS = ("instale", "baixe", "copie a dll", "substitua", "download", "install this")
