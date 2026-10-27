# Validação da v0.5 — Resolution Intelligence

Este registro descreve a entrega original. A correção posterior de aplicabilidade e a expansão de operações DXVK têm relatório próprio em [ACTIONABLE-FIXES.md](ACTIONABLE-FIXES.md); os números abaixo não substituem a validação dessa etapa.

Data: 27/10/2026. No momento da validação, a implementação estava no working tree, sem commits, tags, push ou release. Este documento distingue a base funcional da Resolution Intelligence das classes que continuam exigindo revisão. Não representa validação de ganho real de FPS ou funcionamento de recursos em jogos reais.

## Estado inicial e preservação

Repositório confirmado em `/home/g/Documentos/FRX`, branch `main`, HEAD `b28d237`. A execução começou com 58 arquivos modificados e 14 novos da etapa Hardware Intelligence + Automatic File Fix Engine, ainda não commitados. O índice estava vazio. O diff e a lista inicial foram guardados em `output/resolution-v05/`, ignorado pelo Git; nenhuma alteração anterior foi descartada.

Foram reutilizados Game Discovery, Hardware Context, Fix Discovery/HTTP/cache, Dynamic Fix Engine, regras de segurança e proveniência, Smart Packs, Performance Lab, prévia, confirmação, journal, backup e rollback. A nova camada não tem executor próprio nem transforma prosa externa em comandos.

## Entrega funcional

- Perfil técnico com estados independentes de hardware, jogo, ambiente, configuração e execução.
- Diagnósticos dos sintomas suportados, evidências e hipóteses com limites explícitos.
- Inspeção de arquivos limitada, sem seguir links; bibliotecas representam indícios, não recursos ativos.
- Importação explícita de log UTF-8, AppID obrigatório, até 2 MiB, com persistência de observações padronizadas e digest; não do texto bruto.
- Pesquisa em projetos técnicos primários e issues atribuídas ao AppID, cache/ETag/rate limit, funcionamento offline com dados disponíveis e proveniência separada por tipo.
- Planos de descobertas atribuídas ao jogo que reutilizam as receitas permitidas do Fix Engine; planos incompletos continuam em revisão.
- Preparação do diagnóstico Proton via receita existente `PROTON_LOG=1`, com prévia, consentimento, backup, aplicação e rollback.
- Planos revisáveis para mecanismos de atualização DLSS/FSR documentados pelo GE-Proton; versão incompatível não recebe autorização automática.
- Smart Resolution Packs limitados à composição existente, mesmo jogo e problema. Não são criados pacotes para preencher categorias.
- Resolution Center real na página de cada jogo, com fontes, detalhes, prévia, verificação da alteração e restauração. Modo demo separado.
- Persistência por AppID. Relatório salvo é reavaliado no ambiente atual; fingerprint alterado e divergência no disco impedem confirmação indevida.

## Falhas encontradas e corrigidas

A auditoria identificou risco de apresentar um relatório salvo como estado atual após alteração externa. O carregamento agora recalcula o contexto e confere o estado da operação. A associação também compara o fingerprint da receita: uma fonte alterada não herda a verificação da operação anterior.

A oferta de rollback foi separada da elegibilidade para reaplicar a receita. Uma mudança de fonte/hardware não esconde a restauração de uma operação registrada. O Fix Engine continua revalidando a prévia de rollback e protegendo alterações externas. Quando a mudança de hardware impede gerar a receita atual, o vínculo verificável por `finding_id` e AppID conserva a operação anterior como divergente e oferece sua restauração, sem transferir a confirmação antiga para o plano novo. Esse caso recebeu um teste adicional e nova validação da suíte completa.

Foram corrigidos o vínculo profundo ao detalhe de Smart Pack, o tratamento de datas inválidas vindas de fontes e a persistência dos sintomas selecionados após reinício. Respostas atrasadas não atravessam a troca de jogo.

Os testes de integração também confirmaram que prévia rejeitada não pode ser reutilizada e que uma receita NVIDIA permanece em revisão quando falta identificar o requisito de GPU. As fixtures foram corrigidas para representar esses contratos, sem enfraquecer as validações.

## Testes automatizados

Checks finais do código funcional:

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` | Passou |
| `cargo clippy --all-targets -- -D warnings` | Passou, sem warnings |
| `cargo test` | **370 aprovados, zero falhas, 7 ignorados** |
| `npm test` | **66 aprovados**, 11 arquivos |
| `npm run typecheck` | Passou |
| `npm run build:web` | Passou |
| `npm run build` | Passou: binário Linux, AppImage, .deb e .rpm 0.5.0 |

Os sete testes ignorados são optativos de ambiente/rede/integração; não foram contados como aprovados. Não foram habilitados testes que usam a Steam ou logs reais. A integração TLS local do File Fix Engine foi validada na etapa anterior, não repetida aqui. O build web mantém um aviso de tamanho do chunk principal (~747 kB); o ambiente de testes DOM informa dimensões de gráfico Recharts sem layout real, sem falha.

Após esses checks e o fluxo nativo isolado, os metadados foram atualizados para **0.5.0**. O build final também verificou o TypeScript após ajustar o rótulo de versão da sidebar. O empacotamento Linux final foi concluído: `FRX_0.5.0_amd64.AppImage` (110,51 MiB), `FRX_0.5.0_amd64.deb` e `FRX-0.5.0-1.x86_64.rpm` (4,77 MiB cada), além do binário `src-tauri/target/release/frx`. Não houve instalação dos pacotes no sistema. Logs locais em `output/resolution-v05/` não integram o histórico Git.

A suíte específica da Resolution Intelligence passou com **44 testes**, cobrindo 18 de detecção, 12 de pesquisa, nove de integração/aceitação e cinco de persistência segura. As fixtures são sintéticas e não são gravadas como benchmarks reais. Incluem:

- Fonte geral versus evidência runtime; GPU desconhecida/múltipla; RTX 4060 FG/MFG; FSR não exclusivo da AMD.
- DLL estática, dados incompletos, observação histórica, conflito componente/falha e logs malformados/redigidos.
- AppID incorreto, cache/reinício entre jogos, rate limit, offline, limites HTTP, conteúdo hostil e proveniência de releases sem baixar assets.
- Plano de diagnóstico e solução NVAPI sintética: fonte → diagnóstico → plano → prévia → confirmação → aplicação → journal → verificação → persistência/reinício → rollback.
- Escopo aninhado, registro ilegível preservado, symlinks, atualização de fonte e divergência externa sem falsa verificação.
- Opções GE documentadas ficam em revisão, com distinção entre versão GE correspondente e Proton Valve.

## Validação nativa isolada

Foi aberta uma janela Tauri real, com Vite e processo próprios, `HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, raiz Steam e `/proc` de testes isolados. Duas instalações fictícias foram usadas; a segunda biblioteca fica fora da raiz Steam fictícia. O hardware do sistema foi somente lido. Nenhum jogo real foi iniciado.

O fluxo de `PROTON_LOG=1` passou por diagnóstico, prévia, consentimento, aplicação, conferência da configuração, reinício, persistência, prévia de rollback, confirmação e restauração. Os arquivos originais da fixture voltaram aos mesmos hashes; `gamescope`, `%command%`, opções preexistentes e o segundo jogo foram preservados. A troca de jogo não herdou sintomas ou relatórios.

A interface foi inspecionada em **1440×900** e **1024×760**, preservando identidade, mascote e componentes. As capturas locais estão em `output/resolution-v05/tauri-resolution-1440.png`, `tauri-plan-1024.png` e `tauri-consent-1024.png`. São imagens de uma instalação fictícia, não de benchmarks reais. O cancelamento do seletor nativo não importa nenhum log.

O binário debug final **0.5.0** foi reiniciado após remover externamente somente `PROTON_LOG=1` da instalação fictícia. Sem clicar em Diagnosticar, o carregamento apresentou divergência no disco, não repetiu a confirmação antiga e manteve **Desfazer** disponível. A captura `tauri-final-drift-1024.png` e o relatório `native-validation.json` registram esse teste. Todos os arquivos originais fictícios foram novamente conferidos por hash. Apenas os processos FRX/Vite criados para validação foram encerrados.

Uma revisão independente de quatro capturas e dos fluxos de escopo, URLs, prévia e rollback não encontrou defeitos materiais de interface. O fluxo completo inicial usou o primeiro binário debug; o último binário revalidou especificamente carregamento/estado alterado. A janela nativa não realizou pesquisa pública nem download binário: rede e parsing foram verificados separadamente por testes e GETs de documentação, sem instalação real.

## Batman/PhysX e recursos gráficos

A pesquisa encontrou pistas verificáveis em Proton #309, GE-Proton #196, nvidia-libs e documentação CachyOS. O arquivo histórico instalado pelo usuário permanece não identificado. Não foi criado um patch ou pack alegando correção validada. Nenhuma medição informal foi inserida no Performance Lab.

O mecanismo encontra e explica fontes, requisitos e impedimentos. DLLs, TAR, procedimentos de prefixo/Proton e atualizações binárias sem política de reversão específica permanecem em revisão. Detalhes e fontes: [investigação Batman/PhysX](BATMAN-PHYSX-INVESTIGATION.md) e [diagnósticos gráficos](GRAPHICS-PHYSICS-DIAGNOSTICS.md).

## Limites e pendências

A base funcional permite investigar, aplicar operações já suportadas e conferir/restaurar seus efeitos. Não confirma automaticamente DLSS, PhysX por GPU, ray tracing ou frame generation funcionando. Não seleciona outra versão de Proton nem instala DLLs arbitrárias. A ausência de solução segura é apresentada como resultado válido.

Ainda faltam adaptadores de instrumentação runtime específicos e políticas reversíveis para novas classes binárias. O teste real de Batman depende de participação e autorização futuras. Não há medições novas de desempenho nesta execução. Uma configuração aplicada/verificada não é anunciada como problema resolvido.

## Como usar

1. Execute `npm run dev` e abra um jogo da biblioteca real.
2. No Resolution Center, marque o sintoma e use **Diagnosticar** ou **Pesquisar soluções**.
3. Leia evidências, requisitos e riscos. Quando houver plano aplicável, abra **Visualizar prévia** e confirme somente as mudanças desejadas.
4. **Verificar alteração** confere a configuração. Reproduza voluntariamente o cenário para verificar o recurso; o FRX não inicia o jogo por diagnosticar.
5. Para logs, escolha explicitamente o arquivo correspondente ao AppID. Para desempenho, use o fluxo existente do Performance Lab/Smart Packs com baseline comparável.
6. Se necessário, **Desfazer** abre outra prévia e exige confirmação.

## Git e divisão sugerida

A validação foi concluída antes dos commits. A divisão sugerida para revisão era separar primeiro a base anterior (hardware, descoberta de artefatos, operações seguras por arquivo e respectivas integrações/testes/docs). Depois separar a v0.5 em:

1. `feat(resolution): add contextual technology diagnostics and research`
2. `feat(resolution): integrate persisted plans with fix engine verification and rollback`
3. `feat(ui): add game resolution center and scoped smart pack navigation`
4. `docs: document resolution capabilities and isolated validation`
5. `chore: set validated application metadata to 0.5.0`

Testes devem acompanhar os módulos correspondentes; registro Tauri/modelos/serviços ficam junto à integração que os utiliza. A divisão final deve manter cada commit compilável e distinguir alterações anteriores nos arquivos compartilhados. Não criar um commit artificial só para cumprir essa lista.

## Inventário do working tree

O estado atual contém **99 caminhos pendentes**, incluindo os **72 anteriores**. A lista abaixo diferencia os caminhos novos desta etapa dos arquivos compartilhados; o inventário integral da base anterior permanece em [VALIDACAO-HARDWARE-FILE-ENGINE.md](VALIDACAO-HARDWARE-FILE-ENGINE.md).

Caminhos que passaram a ter alterações nesta etapa:

```text
.gitignore
docs/GRAPHICS-PHYSICS-DIAGNOSTICS.md
docs/RESOLUTION-INTELLIGENCE.md
docs/VALIDACAO-v0.5.md
package-lock.json
package.json
packaging/arch/PKGBUILD
src-tauri/capabilities/default.json
src-tauri/src/commands/mod.rs
src-tauri/src/commands/resolution.rs
src-tauri/src/resolution/acceptance_tests.rs
src-tauri/src/resolution/detection.rs
src-tauri/src/resolution/mod.rs
src-tauri/src/resolution/models.rs
src-tauri/src/resolution/persistence.rs
src-tauri/src/resolution/plans.rs
src-tauri/src/resolution/research.rs
src-tauri/tauri.conf.json
src/components/resolution/ResolutionCenter.tsx
src/layouts/Sidebar.tsx
src/pages/FindingsPage.tsx
src/services/opener.ts
src/services/platform.ts
src/services/resolution.ts
src/test/resolution.test.tsx
src/test/smartPacks.test.tsx
src/types/resolution.ts
```

Arquivos já pendentes que receberam integração da v0.5:

```text
README.md
src-tauri/Cargo.toml
src-tauri/Cargo.lock
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
src-tauri/src/fix_engine/tests.rs
src/components/smart/SmartPacksPanel.tsx
src/pages/SteamGameDetail.tsx
docs/AUTOMATIC-FILE-FIX-ENGINE.md
```

O índice permanece vazio. Builds, logs de execução, screenshots locais, caches, dados da Steam fictícia e backups de testes ficam ignorados. A inspeção dos caminhos versionáveis não encontrou padrões de chave privada/token nem arquivos de logs, staging ou backups reais. Não houve execução de jogo real, escrita na Steam real, alteração de drivers ou configurações globais.

## Estado final

Ao final da validação, `main` estava em `b28d237`, com **99 arquivos pendentes** (incluindo a base anterior preservada) e **nenhuma alteração no índice**. Posteriormente, a base anterior e o Resolution Center foram registrados nos commits locais `c570fab` e `a5f7957`; a documentação foi registrada separadamente. A v0.5.0 estabelece uma base funcional validada em ambiente isolado; correção automática e comprovação runtime de todas as tecnologias não fazem parte do suporte entregue.

O smoke do **último binário de release 0.5.0**, após o ajuste final de rollback, passou sem Vite: interface embutida e backend responderam, a divergência persistida foi detectada ao abrir o jogo e a restauração ficou disponível. O teste foi somente leitura; a fixture permaneceu intacta e o processo criado foi encerrado. Evidência local: `output/resolution-v05/release-smoke-validation.json`. SHA-256 conferido do binário: `25ef3af90f50310348b442d6ed4dcb2b20ac84223e0b282f48a23cdbbf0336cf`.
