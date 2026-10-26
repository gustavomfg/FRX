# Validação — FRX Smart Packs (ciclo 0.4.0)

Validação de 26/10/2026, em CachyOS com Hyprland, RTX 4060 (driver 615.78.08), Ryzen 5 4600G, Steam aberta e Batman: Arkham City GOTY (200260) com proton-cachyos-11.0-20261015.

- Nenhum commit ou tag foi criado.
- Nada foi aplicado na Steam real. O hash do `localconfig.vdf` real permaneceu igual antes e depois dos testes.
- A sessão real do Performance Lab não foi alterada: `smartPack` continua ausente.

> **Sintético × real.** Todo número marcado como *sintético* foi escolhido para o teste. Os únicos dados medidos são uma sessão real de benchmark do Batman e as descobertas reais do cache da pesquisa.

## Verificações automatizadas

| Comando | Resultado |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo test` | **209 aprovados**, 5 ignorados (antes: 180 e 4) |
| `npm test` | **36 aprovados** em 8 arquivos (antes: 29) |
| `npm run build` | ok: AppImage 110,06 MiB, .deb 4,29 MiB, .rpm 4,30 MiB |
| `npm run build:arch` | ok: `frx-bin-0.4.0-1-x86_64.pkg.tar.zst` |

### Testes Rust novos

`smart_packs/tests.rs` tem 24 testes, todos com sessões sintéticas de valores conhecidos.

**Composição:**
- uma receita;
- receitas compatíveis com evidência forte, que viram um único pacote;
- receitas duplicadas;
- contradições e exclusões: mesma variável com valores diferentes, NVAPI ligada × desligada, WineD3D × sem D3D11;
- GPU NVIDIA × AMD;
- versões diferentes do Proton;
- receita já incluída no Proton, limitador de FPS e relato único, que nunca são combinados;
- medição local que torna combinável;
- 40 receitas, que respeitam os limites em menos de 500 ms;
- combinação parcialmente insegura, recusada inteira;
- notas sobre gamescope e MangoHud.

**Ranking:**
- sem benchmarks, não há vencedor e o próximo teste é o baseline;
- 1 × 1 sessão é só indicativo;
- 2 × 2 sessões com melhora de +20,2% e percentuais exatos;
- FPS médio maior com 1% low menor, avaliado em cada perfil;
- frametime instável;
- desempenho semelhante, com empate técnico;
- regressão comprovada;
- condições diferentes, excluídas;
- metadados ausentes, que geram nota;
- dispersão grande, que leva a inconclusivo;
- atualização após novas sessões, com histórico;
- estatística exata;
- sessões impróprias ou sem registro.

**Persistência:**
- migração de formato sem versão;
- formato mais novo, que não é sobrescrito;
- arquivo corrompido, que é guardado;
- pacote aposentado;
- impressão digital que muda com a receita;
- sessão antiga sem `smartPack`.

`fix_engine/tests.rs::packs` tem 5 testes, rodando numa Steam falsa:
- pacote de 2 receitas aplicado como uma operação, com um backup e rollback byte a byte;
- estados `verified` → `rolled_back`;
- troca A → B passo a passo, mantendo a variável compartilhada, e volta ao baseline;
- mudança entre a prévia e a confirmação (descoberta sumiu, relato contrário, registro adulterado), que é recusada sem gravar;
- falha parcial que leva a `partially_recovered` e rollback;
- passo manual com a Steam aberta que continua `testing` após reiniciar;
- ligação das sessões aos pacotes e declaração só para sessões sem registro.

### Testes de front-end novos

`src/test/smartPacks.test.tsx` tem 7 testes:
- sem evidência: nenhum “#1”, nenhum número fictício, ordem de teste e “Referência Windows indisponível”;
- troca de perfil, que recalcula no núcleo, e filtro do ranking medido;
- vencedor medido com destaque, números do relatório, detalhes com a justificativa e “Aplicar” que abre a prévia sem aplicar;
- comparação A × B e “Restaurar estado anterior” passo a passo;
- declaração de sessão e erro do núcleo;
- modo demonstrativo, que não consulta o núcleo;
- Performance Lab mostrando o Smart Pack da sessão.

## Ponta a ponta isolado (dados reais + fixtures identificadas)

Comando: `cargo test real_smart_packs_end_to_end -- --ignored --nocapture`.

Entradas:
- as descobertas reais do Batman, lidas do cache;
- **duas descobertas de fixture** (`fixture-esync`, `fixture-heap`, cada uma com 2 relatos favoráveis);
- uma cópia do `localconfig.vdf` real (com MangoHud);
- o log MangoHud real reprocessado do CSV bruto e declarado como baseline;
- sessões **sintéticas** para os pacotes.

Resultado:

1. **Pacotes compostos:** baseline; “Atrasar liberação de memória” (fixture); “Desativar esync” (fixture); “Desativar Direct3D 11” (descoberta real, um relato); “Pack de compatibilidade” (as duas fixtures).
2. **Fora dos pacotes, com motivo:** winetricks `d3dcompiler_43`, `d3dx9_43` e `dotnet20` (exigem validação); WineD3D (2 relatos contrários); `PROTON_USE_WINED3D11` (variável desconhecida).
3. **Ranking com 1 baseline real e nenhum pacote medido:** “Ainda não há dados suficientes para recomendar um vencedor.” `PROTON_NO_D3D11` ficou como *Experimental — ainda não testado*, sem posição.
4. **Pacote A aplicado na cópia:** `PROTON_NO_ESYNC=1 PROTON_HEAP_DELAY_FREE=1 MANGOHUD_CONFIGFILE=… mangohud %command%`. O backup tem o mesmo SHA do original, e o snapshot do benchmark aponta para o pacote A.
5. **1 sessão real × 1 sintética (+10%):** *Resultado inconclusivo*. A variação natural da sessão real abrange a diferença sintética.
6. **2 × 2 (sintéticas):** o pacote A vai para o ranking medido, #1, com o resumo “#1 — Melhor equilíbrio medido”. O histórico ganhou pontos.
7. **Troca A → B (`NO_D3D11`):** remove as duas variáveis de A e adiciona `PROTON_NO_D3D11=1`, em 2 passos.
8. **A × B (sintéticas a −20%):** “B teve FPS médio menor que A além da variação medida”. No perfil desempenho, B ficou em **Regressão medida**, nos não recomendados.
9. **Rollback final:** cópia idêntica ao original, byte a byte. Histórico do ranking com 5 pontos.

## App Tauri (debug, ambiente isolado)

O ambiente usou HOME, XDG, `FRX_STEAM_ROOT` e `FRX_DEV_PROC_ROOT` apontando para cópias no scratchpad. A sessão real foi **copiada** para os dados isolados. Só descobertas reais.

1. **Fix Packs → Smart Packs:** perfis, filtros, resumo com o Pim, próximo teste, baseline “Ativo”, “Desativar Direct3D 11” como *Experimental — ainda não testado*, 5 não recomendados com o motivo, critérios do perfil, “Referência Windows indisponível” e histórico.
2. **Sessões sem configuração registrada:** a sessão real foi declarada como baseline pela interface, na cópia. Resultado: baseline com 1 sessão; próximo teste “Repita o baseline (1 de 2)”; histórico “Sessões comparáveis: 0 → 1”.
3. **Aplicar “Desativar Direct3D 11”:** prévia com diff, fonte, efeito e risco → confirmação → aplicada. O card ficou como “Ativo · conferido · configuração atual”, e o baseline como “Inativo”, com “Restaurar”.
4. **Restaurar estado anterior:** o diálogo mostrou “remove `PROTON_NO_D3D11=1`” e o passo “Desfazer” → “Desfeita e verificada”. O arquivo isolado voltou ao hash original.
5. **Reinício do app:** registro e histórico persistidos. No Performance Lab, a sessão mostra “Smart Pack: Baseline (nenhuma variável do FRX) · declarado por você”.
6. **Página do jogo:** o resumo Smart Packs aparece ali. Corrigida durante a validação: a contagem de não recomendados ignorava as descobertas fora dos pacotes (mostrava 0, agora 5).
7. **Responsividade:** em 1024 × 760 os controles quebram de linha, sem sobreposição.

O binário de release foi gerado pelos pacotes acima.

## Ajustes feitos durante a validação

- Texto do card do baseline: “nenhuma correções” virou “nenhuma correção”.
- Contagem de “não recomendados” na página do jogo.
- Resultados limitados agora dizem “indício de melhora/piora”, não “melhora/piora”.
- Desempate do estado do pacote quando duas operações têm o mesmo horário: usa o ID da operação, que é crescente no tempo.

## Limites desta validação

- Nenhuma captura nova foi feita: o jogo não foi aberto e a Steam real não foi alterada. Os resultados por pacote usam sessões **sintéticas**; só o baseline é real (e com uma sessão, então tudo fica indicativo).
- Só uma descoberta real do Batman é aplicável (`PROTON_NO_D3D11`, um relato de 2018). Por isso as combinações reais foram demonstradas com duas descobertas de fixture.
- A interação no app foi feita por teclado (o clique sintético não chega à janela no Hyprland).

## Como testar na instalação real (quando você autorizar)

1. Performance Lab: capture **duas** sessões sem correções do FRX, no mesmo trecho e com as mesmas configurações gráficas.
2. Fix Packs → Smart Packs → escolha um candidato → **Aplicar** (com a Steam aberta, siga o passo manual).
3. Capture **duas** sessões com o pacote, no mesmo trecho.
4. Volte a Smart Packs: o pacote entra no ranking medido como melhora, sem diferença ou regressão, com os limiares usados.
5. **Restaurar estado anterior** desfaz a correção.

## Correção: detecção automática de resultados (26/10/2026)

**Causa da regressão não aparecer.** Esta investigação foi feita só em leitura dos dados reais.
- Uma correção foi aplicada e posteriormente revertida.
- `benchmarks/sessions/` estava vazio: nenhuma captura foi feita com o pacote nem sem ele.
- Sem medição, o FRX não tinha o que comparar. O botão do card só abria o Performance Lab, sem fluxo, sem associação e sem trazer o resultado de volta.
- Mesmo com capturas, uma comparação 1 × 1 aparecia só como texto de candidato, sem destaque.

**Implementado:**
- o fluxo “Testar desempenho”;
- a avaliação automática por sessão (`evaluate_benchmark_session`);
- a associação na captura (`tag_assisted`);
- resultados preliminares e comprovados;
- o estado de teste nos cards, o “Último teste” e o resultado no Performance Lab;
- “Restaurar configuração anterior” a partir do resultado;
- o bloqueio de combinações com indício de regressão.

Os detalhes estão em [`SMART-PACKS.md`](SMART-PACKS.md#testar-desempenho-detecção-automática-de-resultados).

### Verificações

| Comando | Resultado |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo test` | **213 aprovados**, 6 ignorados (antes: 209 e 5) |
| `npm test` | **40 aprovados** (antes: 36) |
| `npm run build` | ok: AppImage 110,04 MiB, .deb e .rpm 4,31 MiB |

**Testes novos:**
- Rust (`smart_packs/tests.rs::acceptance`):
  - 1 × 1 vira regressão preliminar, com valores absolutos e percentuais e limiar pela variação natural;
  - 2 × 2 vira regressão comprovada e “não recomendado” nos perfis desempenho e equilibrado;
  - referência, sem baseline, condições diferentes, sem diferença, sinais mistos e configuração desconhecida;
  - indício de regressão bloqueia combinações.
- Front-end:
  - fluxo sem referência, com captura pré-preenchida;
  - captura concluída (evento real `frx://capture`) analisada sozinha, com valores, “Preliminar” e “Restaurar” que abre a troca para o baseline;
  - “Último teste” e estado do card;
  - resultado no Performance Lab.

### Teste de aceitação — Batman (fixtures identificadas)

Comando: `cargo test acceptance_test_performance_batman -- --ignored --nocapture`.

As entradas são uma cópia do `localconfig.vdf` real numa Steam em tempdir e logs MangoHud **de fixture** de 60 s. Os logs foram gerados a partir da sua observação informal (referência ≈ 95 FPS, `PROTON_NO_D3D11=1` ≈ 55 FPS) e processados pelo mesmo caminho da captura assistida. Não são benchmarks reais.

| Etapa | Resultado |
|---|---|
| Referência 1 | “Referência (baseline) registrada”, próximo passo: capturar mais uma |
| Aplicar pacote → captura 1 | associada sozinha ao pacote; **“Indício de regressão — recomendado repetir o teste”**: FPS médio 94,5 → 54,7 (−42,1%, limiar ±3,0%), 1% low 62,5 → 36,4 (−41,7%); faltam 1 + 1 sessões; ranking: candidato “Regressão indicativa” |
| Captura 2 do pacote, restauração e referência 2 | rollback byte a byte; **“Regressão de desempenho medida”**: FPS médio 93,0 → 55,8 (−40,0%, limiar ±3,8%), 1% low 61,9 → 37,1 (−40,1%), P99 12,1 → 20,1 ms; **não recomendado** em desempenho e equilibrado, sem vencedor |

### App Tauri isolado

O app usou cópias: Steam em scratchpad, `FRX_DEV_PROC_ROOT` e as sessões de fixture acima no histórico isolado.

1. **Smart Packs com 1 + 1 sessão:** “Último teste” mostra “Regressão observada · Preliminar” com a tabela referência × pacote × diferença × limiar. O card mostra “Regressão observada · preliminar”.
2. **Testar desempenho:** etapas com a referência já concluída → “Revisar e aplicar” → prévia → aplicado na cópia. O wizard avançou sozinho para “Capturar com o MangoHud”. “Preparar captura” abriu a captura com o aviso do teste e o rótulo “Smart Pack · Desativar Direct3D 11”. O jogo não foi aberto.
3. **Com as sessões de repetição:** “Regressão de desempenho medida”, o efeito “vai para não recomendados” e “Restaurar configuração anterior”.
4. **Restaurar:** diálogo “remove `PROTON_NO_D3D11=1`” → Desfazer → “Desfeita e verificada”. A cópia voltou ao hash original. O pacote está em “Não recomendados”.
5. **Performance Lab:** a sessão do pacote mostra “Smart Pack · resultado”, com a conclusão e o atalho para Smart Packs.
6. **Ajuste encontrado:** o rótulo “2 sessãoões” foi corrigido para “2 sessões”.

### Dados reais

- Nenhuma escrita na Steam real nem no histórico real.
- A Steam atualizou seus próprios dados antes desta validação. As opções de inicialização do jogo permaneceram intactas.

### Limites

- Nenhuma captura real foi feita nesta validação: os números de desempenho são de fixtures.
- O FRX não reconhece o trecho do jogo; a comparação assume o mesmo trecho e as mesmas configurações, e isso aparece em todo resultado.
- A conclusão no Performance Lab é a avaliação atual do pacote, com todas as sessões comparáveis, não só a sessão aberta.
