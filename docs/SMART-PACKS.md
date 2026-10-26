# FRX Smart Packs

Smart Packs são combinações de correções que o FRX compõe sozinho para um jogo. Ele as classifica pelo que foi **medido** no seu computador. Nenhum JSON é escrito à mão. Nenhum FPS é inventado. Quando não há dados, não há vencedor.

Código: `src-tauri/src/smart_packs/`. Comandos: `src-tauri/src/commands/smart_packs.rs`. Interface: `src/components/smart/`.

## Arquitetura

| Camada | Módulo | Papel |
|---|---|---|
| Pack Builder | `discovery`, `composition` | receitas aplicáveis do jogo → pacotes candidatos |
| Compatibility Engine | `conflicts`, `lifecycle` | grafo de conflitos; estado operacional pela prévia do Fix Engine |
| Evidence Engine | `evidence` | sessões do Performance Lab agrupadas por configuração, filtradas por comparabilidade |
| Ranking Engine | `ranking` | ranking medido × candidatos × não recomendados, por perfil |
| Benchmark Integration | `benchmark` | snapshot da configuração em cada sessão; declaração para sessões antigas |
| Persistência | `persistence` | `fix-engine/smart-packs/<appid>.json` |

`smart_packs::build` liga tudo:

1. o gerador dinâmico analisa as descobertas (`FixEngine::analyze`);
2. as receitas aplicáveis viram unidades;
3. as sessões são agrupadas;
4. os pacotes são compostos e registrados;
5. a prévia de cada pacote vem do Fix Engine;
6. o ranking é calculado;
7. o histórico é salvo.

## Unidades (o que pode entrar num pacote)

- Receitas **geradas** a partir das descobertas do jogo, ou **revisadas** do catálogo quando uma descoberta do jogo cita exatamente o mesmo ajuste.
- Receitas genéricas do catálogo sem relação com o jogo não entram, e diagnóstico (`PROTON_LOG`) também não.
- Descobertas contestadas, incompatíveis, já incluídas no Proton ou que exigem validação (winetricks, DLL, registro, arquivos, wrappers, argumentos) ficam em **Não recomendados**, com o motivo.
- Cada receita é uma unidade atômica: nunca é dividida nem misturada com ações de outra.
- Receitas com as mesmas variáveis são deduplicadas.

## Composição (determinística e limitada)

1. **Baseline**: nenhuma correção do FRX.
2. **Receitas individuais**, até `maxUnits` (padrão 8), ordenadas por evidência, risco, nº de variáveis e ID. As que passam do limite ficam fora, com o motivo.
3. **Pacotes por perfil**: união gulosa de receitas *elegíveis* e compatíveis entre si, até `maxPackSize` (padrão 4).
   - **Compatibilidade**: contornos com evidência documentada ou confirmada por vários relatos.
   - **Desempenho**: receitas que, medidas sozinhas, melhoraram o FPS médio sem piorar o 1% low (e sem penalidade documentada).
   - **Equilibrado**: receitas que, medidas sozinhas, não pioraram FPS médio nem 1% low e melhoraram um deles.
4. **Pares** de receitas elegíveis sem conflito conhecido.
5. Total limitado a `maxPacks` (padrão 12). Pacotes com as mesmas variáveis contam uma vez.

**Elegível para combinação** = evidência documentada ou confirmada, **ou** medição local conclusiva sem regressão. Um comentário isolado, ainda não medido, só aparece sozinho: comentários diferentes não são somados às cegas. Limitadores de FPS nunca entram em combinações, porque mudam as condições do benchmark.

O custo é O(n²) no número de receitas, com n ≤ 8. Com 40 receitas, a composição leva menos de 1 ms no teste.

## Conflitos (`conflicts.rs`)

Cada par de receitas recebe uma aresta. As exclusões são:

| Tipo | Regra |
|---|---|
| `same-key-different-value` | mesma variável, valores diferentes |
| `redundant` | uma contém todas as variáveis da outra |
| `mutually-exclusive` | pares listados: WineD3D × sem D3D11/D3D10, ativar × desativar NVAPI, ativar NVAPI × ocultar GPU NVIDIA, dois limitadores de FPS |
| `gpu-mismatch` | uma exige NVIDIA e a outra AMD (ou vice-versa) |
| `frame-limiter` | dois limitadores de FPS |
| `proton-versions` | as duas foram relatadas para versões de Proton sem nenhuma em comum |

Fora isso, a aresta é `no-known-conflict`, com o aviso de que **isso não prova que funcionem bem juntas**. Uma composição só vale se *todos* os pares forem compatíveis. Não existe pacote “parcialmente seguro”.

A linha atual de opções de inicialização é lida só para gerar notas. São elas: variável já definida com outro valor, gamescope com `-r` ou MangoHud ativo junto de um limitador de FPS, e gamescope como condição do benchmark. Correção já incluída no Proton → o planejador bloqueia (`already-included`).

## Perfis

| Perfil | Critério |
|---|---|
| **Máximo desempenho** | Métrica principal: FPS médio. Regressão no FPS médio, ou FPS igual com 1% low pior → não recomendado. FPS maior com 1% low menor → *troca*: fica abaixo das melhoras limpas e nunca vira vencedor. |
| **Equilibrado** | Qualquer regressão em FPS médio, 1% low ou frametime P99 → não recomendado. Ordem: 1% low, P99/picos, FPS médio. |
| **Máxima compatibilidade** | Não piorar a estabilidade medida (1% low, P99, picos). FPS menor é aceito e informado. Ordem: evidência documental, risco, nº de sessões que rodaram até o fim. Vencedor só com evidência documentada ou confirmada. |
| **Baseline** | Referência: não é um perfil para escolher, é a base de toda comparação. Voltar a ele = desfazer as correções ativas do FRX. |

Nenhum perfil reduz a qualidade gráfica: as receitas só definem variáveis do Proton. Resolução, preset, VSync e limite de FPS são **condições** do benchmark. Sessões com diferença nelas não são comparadas.

## Metodologia estatística (`evidence.rs`)

- **Configuração da sessão**: o snapshot `smartPack` gravado no fim da captura assistida (variáveis do FRX conferidas no disco); senão `activeFixes` (v0.4); senão desconhecida. Sessões desconhecidas podem ser **declaradas** pelo usuário e ficam marcadas assim.
- **Referência de condições**: a sessão de baseline mais recente com qualidade suficiente.
  - Sessões com diferença grave em relação a ela ficam fora, com o motivo: jogo, GPU, resolução, preset, limite de FPS, VSync, modo de captura ou duração mais de 4× diferente.
  - Proton, driver, CPU e kernel diferentes também excluem a sessão da comparação controlada. O governador diferente gera nota; os dados originais permanecem no histórico.
  - Dados ausentes (resolução, preset) não bloqueiam, mas são avisados.
- **Por pacote e métrica**: mediana das sessões (robusta a uma partida fora da curva), mínimo, máximo e dispersão = (máx − mín) ÷ mediana.
- **Pacote × baseline**: diferença percentual entre as medianas, com limiar:
  - **≥ 2 sessões de cada lado**: o limiar é o maior entre o piso da métrica e a dispersão dos dois grupos;
  - **1 sessão em algum lado (limitado)**: o limiar é o maior entre o piso e a variação natural dentro da sessão (2 × erro-padrão do FPS por segundo, a mesma estimativa da comparação de sessões). O resultado é só **indicativo** e não entra no ranking medido.
  - Pisos: FPS médio 3%, 1% low 5%, 0,1% low 8%, frametime P99 5%, picos/min 20%.
- **Resultado** por métrica: melhora, regressão ou sem diferença além da variação. Não há porcentagem de confiança: com poucos dados, o resultado é “inconclusivo”.
- CPU, GPU, VRAM e temperaturas não entram no ranking. Uso maior de GPU não significa desempenho melhor.

## Ranking (`ranking.rs`)

- **Ranking medido**: pacotes com resultado conclusivo, ordenados por classe (melhora > troca > sem diferença) e, dentro da classe, pela diferença na métrica principal. Diferenças entre pacotes dentro do limiar são **empate técnico**: os pacotes dividem a mesma posição.
- **Vencedor**: só quando o 1º lugar melhorou a métrica principal além da variação e não divide a posição. Senão, o resumo diz “Ainda não há dados suficientes para recomendar um vencedor.” ou “Resultado inconclusivo…”, e aparece o **próximo teste recomendado**:
  1. medir o baseline;
  2. repetir o baseline até 2 sessões;
  3. testar ou repetir o candidato seguinte.
- **Candidatos para testar**: ordem *de teste*, não de desempenho. Os critérios, em sequência, são:
  1. sem indício de regressão;
  2. já com dados (para completar a repetição);
  3. evidência;
  4. risco;
  5. menos receitas.
- **Não recomendados**: regressão conclusiva para o perfil, incompatíveis e descobertas fora dos pacotes.
- Mais correções não significam mais FPS: o número de receitas nunca melhora a posição.

## Integração com o Performance Lab

- No fim de cada captura assistida, a sessão ganha `smartPack`, com:
  - `packId` e as variáveis do FRX ativas;
  - as receitas e versões;
  - os IDs das operações;
  - `verified`, que indica se tudo foi conferido no disco.

  Proton, driver, hardware, resolução e preset já estavam em `conditions`.
- Sessões antigas não são alteradas. Podem ser declaradas pela interface (`declare_benchmark_pack`), que recusa sessões cuja configuração o FRX já registrou.
- Na página Smart Packs:
  - **Comparar pacotes**: usa a mesma metodologia (A × B);
  - **Ver sessões no Performance Lab**: abre a comparação de sessões existente (`/performance?antes=…&depois=…`).

  Os gráficos e o processamento de logs do MangoHud não foram duplicados.
- **Iniciar benchmark** leva ao fluxo de captura existente. Repetir capturas é o caminho para sair de “indicativo”.

## Aplicação, troca e rollback

- Pacote de uma receita → abre a própria receita no Fix Engine.
- Pacote de várias → receita composta `pack.<appid>.<hash>` (`Trust::Pack`):
  - o núcleo refaz e revalida cada receita de origem;
  - confere que a composição ainda passa no grafo e que o hash bate;
  - aplica tudo numa **única operação**: uma prévia consolidada, um backup verificado, uma gravação atômica e um rollback.

  Não há um segundo executor nem como o pacote ficar “meio aplicado”: se algo falha, o mecanismo de falha e recuperação do Fix Engine vale para o pacote inteiro.
- A impressão digital da prévia cobre o conteúdo da receita composta. Se uma descoberta mudar entre a prévia e a confirmação, nada é gravado.
- **Troca A → B** (`plan_smart_pack_switch`):
  - mostra o que se mantém, o que sai e o que entra;
  - lista os passos: desfazer cada operação ativa, depois aplicar B;
  - cada passo abre a própria prévia e confirmação;
  - operações pendentes (passo manual ou recuperação) bloqueiam a troca até serem resolvidas.
- **Restaurar estado anterior** = trocar para o baseline.
- O rollback remove só o que o FRX adicionou e recusa sobrescrever mudanças de terceiros. Funciona depois de reiniciar, pelo diário.

### Estados operacionais

Os estados são `candidate`, `ready`, `conflict`, `testing`, `applied`, `verified`, `failed`, `partially_recovered`, `rolled_back` e `unsupported`. Eles descrevem execução e verificação, nunca ganho de FPS. A tabela completa está em `lifecycle.rs`.

## Testar desempenho (detecção automática de resultados)

**O que faltava.** O ranking só usava sessões já capturadas e só dava conclusões com 2 sessões de cada lado. O botão do card apenas abria o Performance Lab, sem ligar a captura ao pacote nem trazer o resultado de volta. Assim, alguém podia aplicar um pacote, jogar e desfazer sem que o FRX medisse nada.

**Fluxo guiado** (`src/components/smart/PackTest.tsx`), no botão **Testar desempenho** de cada pacote aplicável. No baseline, o botão é **Capturar referência**.

1. **Referência sem correções.**
   - Se não houver sessão de baseline comparável, o FRX oferece capturá-la.
   - Se houver correções ativas, oferece primeiro **Restaurar configuração anterior**: um passo a passo, cada passo com prévia e confirmação.
   - É possível seguir sem referência; nesse caso, o resultado será “Sem referência para comparar”.
2. **Aplicar o pacote**: mostra as variáveis e abre a prévia do Fix Engine. Nada é gravado sem confirmação. Com a Steam aberta, vale o modo manual de sempre.
3. **Capturar**: abre a captura existente do Performance Lab, já preenchida com o rótulo do teste e as condições da referência (resolução, preset, limite de FPS, VSync).
   - Para jogos Vulkan, **Configurar pelo FRX (Vulkan)** prepara `MANGOHUD=1` e o caminho do arquivo próprio do FRX com a prévia, o backup e o rollback do Fix Engine. O jogo e a Steam precisam estar fechados pelo usuário. A receita interna `frx.capture.<appid>` não vem das descobertas, não aceita caminhos enviados pela interface e não entra nas correções do ranking. A allowlist das receitas externas continua recusando essas variáveis.
   - Quando as opções já habilitam o MangoHud e apontam para o arquivo correto, o assistente oferece **Aguardar o jogo**, sem pedir colagem. A configuração automática é para Vulkan; OpenGL e Flatpak continuam com a alternativa manual e precisam de validação própria.
   - No teste guiado, o início manual vem selecionado para evitar medir menus. O jogo escolhido fica fixo, e a referência recebe seu próprio rótulo.
   - A captura dura só o tempo escolhido, sem monitoramento contínuo.
   - Com início manual, a gravação começa quando você chega ao trecho jogável (pelo FRX ou Shift esquerdo + F2), sem medir menus.
   - O jogo é aberto pela Steam com o mecanismo existente.
4. **Resultado automático**: quando a captura termina, o FRX analisa o log, associa a sessão à configuração ativa e compara com a referência, sem nenhuma ação manual.

**Associação automática.** O fluxo guiado envia o `expectedPackId`; o núcleo confere o pacote ao preparar e ao começar a captura. O FRX registra a configuração no início e confere novamente ao terminar. Se mudar, a sessão é guardada com aviso e excluída do ranking. No fim de toda captura assistida, `smart_packs::benchmark::tag_assisted` grava na sessão:
- as correções do FRX ativas e conferidas no disco (`activeFixes`);
- o Smart Pack equivalente (`smartPack`: variáveis, receitas, operações, `verified`).

O pacote é identificado pelas variáveis, então a sessão pertence ao pacote realmente ativo, seja qual for o caminho de aplicação.

**Avaliação** (`smart_packs/evaluation.rs`, comando `evaluate_benchmark_session`). A mesma metodologia do ranking (medianas, limiares, comparabilidade) compara o pacote com a referência e produz:

| Situação | Conclusão |
|---|---|
| 1 sessão em algum lado | **preliminar**: “Indício de regressão — recomendado repetir o teste”, “Indício de melhoria…”, “Sem diferença relevante detectada (preliminar)” |
| ≥ 2 sessões comparáveis de cada lado | “Regressão de desempenho medida”, “Melhoria de desempenho medida” ou “Sem diferença relevante detectada” |
| FPS médio e 1% low em direções opostas | “Resultado inconclusivo” |
| condições diferentes da referência, qualidade insuficiente ou configuração desconhecida | “Não foi possível determinar a diferença” |
| sem baseline | “Sem referência para comparar” |

Sessões interrompidas, impróprias, com configuração não confirmada ou marcadas como `synthetic`/fixture são preservadas, mas excluídas da evidência real. Esse marcador identifica dados conhecidos de teste: não é um detector capaz de provar a autenticidade de qualquer CSV externo.

Cada conclusão traz:
- valores absolutos (referência → pacote), diferença absoluta e percentual, e o limiar usado, para FPS médio, 1% low, 0,1% low, frametime P99 e picos/min;
- o número de sessões de cada lado;
- as condições de hardware/software registradas e as limitações, entre elas a de que cenas diferentes também mudam o FPS;
- o próximo passo (“faltam N sessões…”);
- o efeito no ranking.

Duas sessões por configuração são um ponto inicial de observação, não prova científica de causalidade. Uma captura só nunca sustenta uma conclusão definitiva. Com 1 sessão, o limiar é a variação natural dentro da sessão, e o resultado é só indicativo.

FPS médio maior com piora em 1% low, 0,1% low, P99 ou picos é tratado como sinal misto. A tabela preserva todas as métricas, e o perfil de desempenho não elege esse trade-off como vencedor. Se o baseline tem zero picos, a diferença absoluta aparece com percentual indisponível, sem dividir por zero.

**Onde aparece:**
- no próprio fluxo, ao terminar a captura;
- no topo de Smart Packs (“Último teste”);
- no estado de cada card: Aguardando baseline, Pronto para teste, Capturando desempenho, Analisando resultados, ou o resultado (com “· preliminar” quando for o caso);
- no Performance Lab, na sessão (“Smart Pack · resultado”), com a conclusão atual do pacote, que usa todas as sessões comparáveis.

**Consequências no ranking:**
- *Preliminar com regressão*: o pacote fica no fim dos candidatos, com “Regressão indicativa — repita para confirmar”, e **não entra em combinações** até ser repetido.
- *Regressão medida em repetições (2 × 2)*: o pacote vai para **não recomendados** nos perfis em que a métrica piorou e nunca é vencedor.
- O histórico de medições e o histórico do ranking são preservados.

**Restauração.** Quando a conclusão indica regressão e o pacote ainda está ativo, aparece **Restaurar configuração anterior**. O botão abre a troca para o baseline: desfazer cada operação, com a prévia do rollback e confirmação. Nada é revertido automaticamente.

## Persistência

`fix-engine/smart-packs/<appid>.json` (pasta 0700, arquivo 0600, gravação atômica) guarda:

- cada pacote composto: ID, versão, receitas, impressões digitais, estado, fontes, conflitos evitados, evidência e requisitos, com `firstSeen` e `lastSeen`; os que deixam de ser compostos ficam `retired`, sem ser apagados;
- o **histórico do ranking**: um ponto por mudança de ordem, por perfil, com o motivo (“Sessões comparáveis: 2 → 4”), até 500 pontos.

Quando algo muda (fontes, receitas, Proton, driver, regras, novas sessões), tudo é recalculado ao abrir a página. O ranking usa só as estatísticas já salvas nas sessões, sem reprocessar logs. Os arquivos são tratados assim:

- sem `schema`: migrado;
- de formato mais novo: não é alterado;
- corrompido: guardado como `.json.corrupt` e refeito.

## Referência Windows

O modelo `WindowsReference` registra:

- origem e URL;
- versão do jogo;
- CPU, GPU e driver;
- resolução e preset;
- FPS médio e 1% low;
- data e condições.

Ele é lido de `smart-packs/windows/<appid>.json` se existir. Nenhuma fonte é integrada e nenhum servidor é usado. Sem dados, a interface mostra **“Referência Windows indisponível.”** Relatos como “roda melhor no Linux” nunca viram número.

## Segurança

- Só variáveis da lista revisada e da base de conhecimento, validadas de novo na receita composta (`security::validate_pack`).
- O catálogo recusa receitas `pack`.
- A interface envia só IDs e a impressão digital da prévia.
- O registro é ponteiro, não fonte de verdade: um membro adulterado ou inexistente faz a prévia falhar.
- Nada é aplicado sem confirmação. Nenhuma sequência de testes roda sozinha.

## Limitações

- O ranking depende de benchmarks locais repetidos: com uma sessão de cada lado, tudo é indicativo.
- Não há dado externo de benchmark integrado, nem referência Windows.
- “Funcionou” no perfil de compatibilidade quer dizer “a captura rodou até o fim sem piorar a estabilidade”. Não prova que um problema relatado sumiu.
- A identidade do cenário (trecho do jogo) depende do usuário repetir o mesmo trecho. O FRX não reconhece cenários.
- As regras de exclusão são uma lista explícita e pequena. Pares fora dela recebem “nenhum conflito conhecido”, nunca “compatível”.

## Testes

- **Rust**:
  - `smart_packs/tests.rs` (24): composição, conflitos, ranking com sessões sintéticas de valores conhecidos e persistência;
  - `fix_engine/tests.rs::packs` (5): aplicação composta, troca, falha e recuperação, passo manual após reinício, sessões e declaração;
  - `real_smart_packs_end_to_end` (ignorado): ponta a ponta com dados reais, em cópias.
- **Front-end**: `src/test/smartPacks.test.tsx` (7).

Detalhes e resultados: [`VALIDACAO-SMART-PACKS.md`](VALIDACAO-SMART-PACKS.md).
