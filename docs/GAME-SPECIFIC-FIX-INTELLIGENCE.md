# Game-Specific Fix Intelligence

Implementação na v0.4.0, 27/10/2026. Não altera a versão do aplicativo.

## Problema encontrado

O projeto já usava arquivos por AppID e IDs `gen.<appid>.*` / `pack.<appid>.*`, mas isso não bastava para isolar a evidência. O provedor GitHub podia atribuir ao jogo consultado uma issue retornada pela busca ou uma nota de versão identificada apenas pelo nome. A deduplicação usava ações iguais sem conferir toda a atribuição. O gerador aceitava `gameAppId` sem uma prova separada da origem; uma ação correspondente a receita revisada também podia ignorar a restrição de hardware da descoberta.

A composição recebia unidades sem identidade explícita de jogo. Associações antigas e snapshots não conferiam integralmente AppID, operação, ações e identidade da receita. Editar o AppID de uma sessão podia conservar a associação anterior. Havia ainda respostas assíncronas e diálogos capazes de mostrar resultados do jogo anterior. Nenhuma dessas constatações demonstra que uma medição real de outro jogo chegou a afetar uma recomendação do usuário; são brechas identificadas no código e cobertas por testes.

A triagem anterior também tratava o risco técnico como um filtro excessivamente geral: um relato específico favorável não podia justificar um candidato com aviso, e a penalidade de WineD3D bloqueava combinações mesmo diante de medições locais favoráveis.

## Três camadas de conhecimento

1. **Global técnico:** `fix_engine/dynamic/compatibility.rs`, allowlist e catálogo descrevem operação, requisitos, reversibilidade e riscos potenciais. Não representam uma medição de FPS nem um veredito universal. O catálogo continua disponível para revisão manual de compatibilidade; sua presença não cria uma recomendação automática para todos os jogos.
2. **Específico do jogo:** `FixFinding.scope` indica `verified-game`, `general-technical` ou `unknown-scope`. Relatos de desempenho ficam em `performanceEvidence.reports`, separados dos votos de “funcionou / não funcionou”. Cada relato registra AppID, ação normalizada, autor quando conhecido, fonte, data, afirmação, resultado, GPU/Proton informados, confiança e limitações. O contexto adicional da descoberta inclui problema, objetivo, requisitos e versão do jogo quando fornecida.
3. **Ambiente local:** sessões do Performance Lab conservam AppID, condições, build Steam conhecido, métricas, qualidade e snapshot do pacote. O ranking considera somente o jogo e o contexto correspondentes. Relatos públicos e observações informais não são convertidos em benchmarks.

Segurança permanece global. Uma fonte favorável não libera scripts, shell, DLLs, downloads, variáveis proibidas ou ações fora do formato permitido. Prévia, confirmação, backup, verificação e rollback continuam passando pelo Fix Engine existente.

## Como a origem confirma o jogo

- **GitHub Issues:** repositório integrado permitido, URL da issue coerente e identidade explícita da fonte. Um número entre parênteses no título é apenas indício; precisa da identificação por `AppID` ou link Steam no título/corpo. Assim `(4060)` ou `(2024)` não viram AppIDs por acaso. A consulta sozinha não comprova vínculo.
- **Comentários:** precisam pertencer à issue verificada. Referências explícitas a AppIDs diferentes e sinais de discussão sobre outro jogo tornam o trecho sem atribuição confirmada. A ligação com a issue original continua na proveniência.
- **Notas de versão:** menção apenas pelo nome é preservada como origem sem atribuição confirmada. Identificação explícita deve corresponder a um único AppID; linhas compartilhadas ambíguas não recebem atribuição otimista.
- **UMU-Protonfixes:** o arquivo solicitado por AppID confirma a origem, inclusive quando um link do repositório reutiliza a implementação de outro arquivo. A reutilização do código não transporta a recomendação do jogo de destino do link.
- **UMU Database:** mantém a associação explícita por ID da base; aliases não inventam um AppID para uma descoberta ambígua.
- **Documentação geral:** explica a tecnologia e os avisos, sem gerar evidência de ganho de FPS ou receita específica por simples consulta.

O parser de relatos é determinístico e conservador. Só interpreta frases explícitas de direção de desempenho na mesma linha da ação reconhecida; “funcionou”, “corrigiu o crash” e “fixed” não significam FPS maior. Não atribui ganho a uma variável quando o texto diz que removê-la melhora o desempenho. Não distribui uma afirmação entre várias ações de uma linha ambígua.

## Avaliação contextual

| Evidência disponível para este AppID | Avaliação |
| --- | --- |
| Só risco técnico global | Risco potencial; impacto neste jogo desconhecido; disponível para compatibilidade quando aplicável |
| Relato explícito de mantenedor, ou pelo menos dois autores independentes favoráveis, sem contradição e sem contexto conhecido incompatível | Possível melhoria específica; candidato para validação local; aviso global permanece |
| Capturas locais comparáveis e consistentes favoráveis | Melhoria observada nesta configuração; participa do ranking conforme o perfil |
| Capturas locais comparáveis negativas | Regressão observada nesta configuração; perde prioridade conforme o perfil |
| Relatos positivos e negativos | Evidências conflitantes; nenhuma seleção automática da interpretação otimista |

A contagem contextual é recalculada dos relatos e das fontes; não confia num contador agregado como prova. Autor desconhecido não estabelece independência entre relatos da comunidade. GPU/Proton conhecidos incompatíveis e versão do jogo não confirmada impedem promoção automática por relato. Dados ausentes não viram afirmação de contexto igual: as limitações permanecem visíveis. Relatos positivos não elegem vencedor e não autorizam por si só combinar alternativas com risco; as combinações de desempenho exigem medições locais adequadas.

Uma regressão do Batman não é regra para outro AppID. Uma melhoria de outro jogo também não promove Batman. Os valores informais relatados pelo usuário não foram registrados como baseline nem usados para calcular percentuais nesta implementação.

## Receitas, composição e identidade

O gerador exige simultaneamente `finding.gameAppId == contexto.appId` e `scope.matches_game(appId)`. Receitas geradas incluem `generated.gameAppId` e mantêm evidência/proveniência próprias. A correspondência com o catálogo respeita o escopo e as restrições da descoberta. A execução regenera a receita a partir da descoberta atual; o registro local não é fonte executável.

`discover(appId, ...)` recebe o jogo explicitamente. Cada `Unit` tem AppID; o compositor e o detector de conflitos recusam unidades estrangeiras ou legadas sem atribuição. Receitas gerais revisadas só participam de pacotes quando uma descoberta verificada do jogo justifica a mesma ação. Os limites de quantidade de receitas, conflitos e tamanho de combinação permanecem.

Há duas identidades complementares:

- `pack.<appid>.<hash>` identifica a configuração exata de ações do jogo. Mantê-la estável preserva a ligação com históricos já existentes.
- O fingerprint versionado do pacote inclui AppID, ações, membros, versões e fingerprints das receitas. Uma operação nova grava o fingerprint exato da receita aplicada. Snapshots de captura incluem identidade explícita e fingerprints das receitas, além de operações, versões e ações.

A associação confere o AppID da sessão, o do snapshot, o derivado do ID do pacote, a união das ações e os registros das operações. Receitas geradas/compostas estrangeiras são recusadas. A revalidação da aplicação recompõe somente descobertas pertinentes ao jogo e confere versões e hashes dos membros. Uma alteração de receita não herda silenciosamente a evidência da composição anterior.

## Cache, persistência e migração

- Descobertas: `cache/fixes/<appid>.json`; o AppID interno deve corresponder à chave. Descobertas estrangeiras não aparecem como específicas do jogo.
- Receitas: `fix-engine/generated/<appid>.json`; valida identidade das entradas e invalida por descoberta, catálogo, GPU/driver, Proton, build conhecido, gerador e regras. Gerador `1.3.0`; regras `2026.10.27-4`. A aplicabilidade é recalculada pelo planejador, não herdada do cache. Veja [ACTIONABLE-FIXES.md](ACTIONABLE-FIXES.md).
- Smart Packs: `fix-engine/smart-packs/<appid>.json`; ponteiros de pacotes são verificados também por conteúdo e membros. Composição e ranking `1.4.0`; configurações mescladas incluem o digest dos bytes resultantes.
- Histórico de aplicação: mantém AppID e, nas novas operações, fingerprint da receita. Rollback usa a operação original, não uma recomendação de outro jogo.
- Performance Lab: `benchmarks/sessions/<id>`; o ID interno deve corresponder ao diretório. Arquivos inválidos ficam preservados no disco, sem serem executados ou reinterpretados como sessão de outro jogo.

Campos novos são aditivos e possuem valores padrão de leitura. Nenhuma migração atribui automaticamente o jogo aberto a um registro legado. Descobertas sem `scope` continuam guardadas, são marcadas para atualização e não geram receitas até uma pesquisa confirmar a origem. Registros gerados antigos são reanalisados pelas novas regras.

Snapshots antigos sem hashes podem conservar a associação quando o AppID, as operações históricas e as ações a sustentam, com limitação explícita; não recebem o hash atual inventado. Registros sem vínculo suficiente continuam sem associação. O histórico não é descartado quando o contexto muda.

## Performance Lab

Preserva Baseline → Aplicar → Capturar → Comparar → Classificar. Só sessões válidas do AppID correspondente influenciam o ranking. Capturas interrompidas, sintéticas, não verificadas, com configuração alterada ou receita divergente são preservadas e excluídas da avaliação automática.

O ranking do ambiente atual confere GPU, driver, Proton, CPU, build conhecido e demais diferenças de contexto registradas. A ausência histórica de um campo crítico conhecido atualmente impede tratá-la como evidência local compatível. A comparação histórica continua disponível com suas limitações. Normalização é restrita a grafias equivalentes do mesmo modelo/driver, sem equiparar GPUs apenas pelo fabricante.

Novas importações não herdam o Proton/hardware de hoje para um CSV antigo. O formulário permite informar Proton e build da captura quando conhecidos. Uma captura assistida não pode ser movida para outro jogo; reatribuir uma importação remove a declaração antiga do pacote. Alterar condições de uma captura registrada conserva seus dados, mas invalida sua participação automática. A reanálise preserva os marcadores de exclusão.

## Interface e validação

Mantidos componentes, cores, personagens, animações e organização existentes. Os cards explicam AppID, atribuição, relatos específicos, risco geral e impacto desconhecido. Respostas assíncronas, detalhes, comparações e prévias são isolados por jogo para evitar mostrar dados da seleção anterior.

Os resultados finais dos comandos, a demonstração Tauri e a relação exata de arquivos estão na seção de validação abaixo. Todos os cenários automatizados usam fixtures identificadas; não são evidência de ganho real de FPS.

## Limitações

- Texto livre ambíguo sem AppID ou sinal identificável de outro jogo não pode ser resolvido semanticamente pelo parser; não há modelo de linguagem atribuindo identidades por semelhança.
- Expressões complexas de desempenho e relatos com múltiplas ações podem ficar sem conclusão automática. A fonte original continua acessível.
- Relatos públicos frequentemente omitem driver, CPU, gráficos e versão do jogo. São candidatos com limitações, nunca medições locais.
- Importações antigas podem conter campos preenchidos pelo código anterior sem proveniência individual; não é possível reconstruir com certeza sua origem. Os dados foram preservados.
- Mudanças de conteúdo/proveniência de receitas com fingerprint novo podem retirar medições anteriores da composição atual de forma conservadora.
- Builds Steam ausentes em registros antigos permanecem desconhecidos. Nenhuma atualização completa retroativamente esses metadados com o jogo atual.
- Um CSV externo adulterado não pode ser autenticado pelo FRX. Arquivos explicitamente sintéticos são excluídos; isso não equivale a atestar autenticidade de qualquer arquivo.
- Esta validação não iniciou jogos reais, não mediu FPS adicional e não substitui medições controladas do jogo do usuário.

## Resultados desta implementação

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` | Aprovado |
| `cargo clippy --all-targets -- -D warnings` | Aprovado, sem warnings |
| `cargo test` | 260 aprovados, 0 falhas, 6 ignorados |
| `npm test` | 49 aprovados em 9 arquivos |
| `npm run typecheck` | Aprovado |
| Build web (TypeScript + Vite) | Aprovado |
| Build Tauri Linux final | Aprovado: binário + AppImage, .deb e .rpm |
| Verificação mecânica dos componentes alterados | Sem problemas determinísticos reportados |

Os seis testes ignorados envolvem pesquisa/varredura real, log pessoal ou cenários manuais anteriores e não foram executados. As verificações desta tarefa não iniciaram jogos, não fecharam a Steam e não escreveram em configurações reais. Os avisos existentes de dimensão zero do Recharts no DOM de teste e de bundle JavaScript acima de 600 kB permanecem; não são falhas dos comandos.

### Casos de regressão cobertos

- Mesma ação com melhoria no jogo A, regressão no Batman (AppID 200260) e nenhuma medição no C, em fixtures sintéticas identificadas.
- Descoberta/issue/comentário de outro jogo, título numérico ambíguo e nota global não promovem receitas do jogo consultado.
- Documentação técnica e votos de compatibilidade não se tornam promessa de FPS.
- Relatos específicos confiáveis podem promover candidato com aviso, sem liberar ação proibida nem combinação de risco sem medição local.
- Relato negativo contestado em descoberta separada é preservado e conflita com a alegação positiva da mesma ação; uma sétima fonte negativa não desaparece por truncamento.
- Composição, persistência, reinicialização, troca de AppID, registros legados, fingerprints adulterados e alteração de receita.
- Contextos locais diferentes ou desconhecidos, inclusive build Steam, não elegem vencedor do ambiente atual.
- Alteração de AppID/condições, reanálise e associação declarada não reaproveitam silenciosamente a configuração anterior.
- Respostas atrasadas, finding IDs compartilhados, detalhes, comparações e prévias não transportam a seleção anterior na interface.
- Backup, aplicação transacional e rollback continuam cobertos pelos testes existentes em Steam fictícia.

### Ponta a ponta no Tauri

Executado no Hyprland/Wayland, com janela GTK/Xwayland, em `HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` e `FRX_STEAM_ROOT` isolados. Duas bibliotecas fictícias, dois jogos com nomes explicitamente sintéticos e **zero sessões de benchmark**:

- **Fixture A / AppID 200260:** `PROTON_NO_D3D11=1` como candidato por relatos específicos favoráveis, sem vencedor nem FPS medido. Detalhes mostram fontes e aviso global.
- **Fixture B / AppID 620:** a mesma ação em risco potencial, impacto desconhecido e revisão de compatibilidade. Nenhum resultado de A aparece como medição de B.
- Dimensões verificadas: 1200×800 e 1024×760. Personagens, cores e animações preservados.
- Conferência SHA-256 dos arquivos Steam fictícios antes/depois: inalterados. Pastas de sessões: vazias.

Essa navegação revelou um defeito preexistente na capa provisória: deslocamento de bits com sinal podia gerar índice negativo para o AppID 620 e deixar a página vazia. Foi corrigido com deslocamento sem sinal e teste de regressão; a navegação foi repetida com sucesso. Não houve alteração artística.

Uma tentativa intermediária de empacotamento falhou por `Text file busy`: a cópia de teste havia sido aberta antes de terminar o ajuste do executável pelo empacotador. A validação passou a executar uma cópia separada do binário, para permitir repetir o empacotamento sem esse conflito.

Capturas e logs de QA ficam apenas em `output/playwright/game-specific/` (ignorado pelo Git). São artefatos de teste, não evidências de desempenho do usuário.

### Estado do Git e revisão

Estado inicial limpo em `main`, cinco commits anteriores à frente de `origin/main`, último commit `aff6a36`. Ao fim da validação inicial, as alterações estavam no working tree para revisão. Depois foram organizadas em commits locais, sem criar tag ou release. Versão do aplicativo mantida em 0.4.0.

Arquivos modificados/criados (57):

```text
README.md
docs/GAME-SPECIFIC-FIX-INTELLIGENCE.md
docs/SMART-PACKS.md
src-tauri/recipes/catalog.json
src-tauri/recipes/frx.compat.wined3d.json
src-tauri/src/commands/fix_engine.rs
src-tauri/src/commands/performance.rs
src-tauri/src/fix_engine/dynamic/analyzer.rs
src-tauri/src/fix_engine/dynamic/compatibility.rs
src-tauri/src/fix_engine/dynamic/game_evidence.rs
src-tauri/src/fix_engine/dynamic/generator.rs
src-tauri/src/fix_engine/dynamic/mod.rs
src-tauri/src/fix_engine/dynamic/registry.rs
src-tauri/src/fix_engine/dynamic/tests.rs
src-tauri/src/fix_engine/mod.rs
src-tauri/src/fix_engine/models.rs
src-tauri/src/fix_engine/security.rs
src-tauri/src/fix_engine/tests.rs
src-tauri/src/fixes/aggregation.rs
src-tauri/src/fixes/compat.rs
src-tauri/src/fixes/mod.rs
src-tauri/src/fixes/models.rs
src-tauri/src/fixes/providers/github.rs
src-tauri/src/fixes/providers/umu.rs
src-tauri/src/performance/comparison.rs
src-tauri/src/performance/mod.rs
src-tauri/src/performance/models.rs
src-tauri/src/performance/persistence.rs
src-tauri/src/smart_packs/benchmark.rs
src-tauri/src/smart_packs/composition.rs
src-tauri/src/smart_packs/conflicts.rs
src-tauri/src/smart_packs/discovery.rs
src-tauri/src/smart_packs/evaluation.rs
src-tauri/src/smart_packs/evidence.rs
src-tauri/src/smart_packs/mod.rs
src-tauri/src/smart_packs/models.rs
src-tauri/src/smart_packs/persistence.rs
src-tauri/src/smart_packs/ranking.rs
src-tauri/src/smart_packs/tests.rs
src/components/fix/ApplyFixDialog.tsx
src/components/fix/FindingCard.tsx
src/components/fix/GameEvidence.tsx
src/components/fix/PerformanceWarning.tsx
src/components/fix/RecipesPanel.tsx
src/components/fix/useAssessments.ts
src/components/performance/SessionForm.tsx
src/components/smart/SessionPackResult.tsx
src/components/smart/SmartPacksPanel.tsx
src/lib/steamGame.test.ts
src/lib/steamGame.ts
src/state/fixes.tsx
src/test/fixtures.ts
src/test/gameSpecific.test.tsx
src/types/fixEngine.ts
src/types/fixes.ts
src/types/performance.ts
src/types/smartPacks.ts
```
