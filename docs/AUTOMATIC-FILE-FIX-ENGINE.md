# Automatic File Fix Engine

Implementação incremental sobre o Fix Engine existente, sem executor paralelo. Base introduzida na v0.4.0 e reutilizada pela Resolution Intelligence na v0.5.0. Esta alteração não aplica patches na Steam real, não inicia jogos, não instala drivers e não executa payloads externos.

Veja também [hardware e evidências](HARDWARE-FIX-INTELLIGENCE.md), [investigação Batman/PhysX](BATMAN-PHYSX-INVESTIGATION.md) e [isolamento por jogo](GAME-SPECIFIC-FIX-INTELLIGENCE.md).

## Suporte efetivamente implementado

| Operação | Suporte inicial |
| --- | --- |
| Descobrir artefatos | Metadados de GitHub Releases referenciadas pelas fontes já integradas. Downloads anexados ou externos sem procedência suficiente permanecem em revisão. |
| Baixar e verificar | HTTPS, staging privado, tamanho e SHA-256 publicados no asset, reconsulta da release por ID e das instruções do publicador antes de baixar o payload. |
| Extrair membros | ZIP, somente membros explicitamente selecionados, com hashes individuais; conteúdo validado em memória, sem extrair no jogo. |
| Instalar/substituir | Dados UTF-8 em `.ini`, `.cfg`, `.conf`, `.json` e `.txt`, com gramática limitada de configuração; substituição exige hash conhecido do original. |
| Patch de configuração conhecida | Ação tipada `PatchKnownConfig`: hash exato do original, substituições literais inequívocas, validação do resultado, backup e rollback. Ainda não é inferida de prosa externa. |
| Restaurar | Rollback pelo journal existente, após confirmação e validação de divergências. |
| DLL, EXE, ELF, scripts, XML e TAR | Requerem revisão. Não são instalados ou executados automaticamente. |
| Prefixos, Wine registry, winetricks, dependências do sistema e componentes Proton | Não automatizados nesta fase. Não há sudo. |

Um `.txt` não é aceito só pela extensão: deve conter dados de configuração suportados. São recusados conteúdo binário, scripts, diretivas de execução/carregamento e instruções não reconhecidas. JSON precisa ser objeto válido; configurações de texto usam seções e pares chave/valor. A política conservadora pode recusar configurações legítimas que precisem de revisão específica.

## Modelo e descoberta

`ArtifactCandidate` conserva AppID e escopo, fonte da recomendação, repositório, publicador, release/ID/tag, asset/ID/nome/URL/tamanho, formato, digest, assinatura quando identificada, licença quando declarada, plataforma, arquitetura, requisitos de hardware/Proton, membros e destinos relativos. Campos desconhecidos permanecem ausentes.

A fonte da recomendação não ganha autoridade para escolher destinos: o mapeamento vem da publicação da release. O parser aceita instruções estreitas e completas nas notas da release ou um bloco tipado `frx-file-fix`. Trata-se de um protocolo genérico do publicador, não de JSON cadastrado pelo desenvolvedor por jogo. Os formatos estão exemplificados na [investigação](BATMAN-PHYSX-INVESTIGATION.md#contrato-de-descoberta-de-arquivos-suportado).

A busca adicional por nome apenas localiza referências. Ela não confirma a identidade do jogo. São exigidos vínculo verificável com AppID e procedimento completo. Comentários, anexos, prosa ambígua, outro AppID, caminhos adivinhados e instruções com etapas externas desconhecidas não geram instalação parcial. Procedimentos envolvendo vários assets ainda ficam em revisão; um ZIP pode conter vários membros selecionados e validados em conjunto.

O gerador dinâmico produz `InstallArtifact` com a mesma infraestrutura de receitas, IDs por AppID, proveniência e fingerprint. A interface não envia uma receita arbitrária para execução: o comando resolve e revalida a receita no núcleo a partir do estado conhecido.

## Política de confiança

Os níveis são `origin-and-integrity`, `partial` e `untrusted`. O primeiro significa procedência identificada e integridade verificável em relação à publicação. **Não significa que o arquivo é benigno, totalmente seguro ou melhora desempenho.** HTTPS protege o transporte; SHA-256 verifica bytes contra a referência publicada; nenhum dos dois certifica o autor ou a finalidade.

A lista inicial de repositórios reconhecidos está centralizada em `fixes/providers/artifacts.rs` e documentada na investigação. Ela limita procedência; não permite instalar seus binários. A aprovação técnica de novas fontes é uma política do FRX, enquanto novas receitas do mesmo protocolo podem ser descobertas sem release do aplicativo.

O executor refaz a consulta no endpoint canônico `https://api.github.com/repos/<owner>/<repo>/releases/<id>`. Confere autor, IDs, tag, URL, estado `uploaded`, nome, tamanho e digest, e regenera o candidato completo com o mesmo parser. Mudança de destino, hashes, requisitos ou instruções invalida a prévia e exige atualizar a descoberta. Cada artefato tem seus próprios requisitos reavaliados antes de preparar destinos, independentemente dos requisitos agregados da receita: build exato quando declarado, GPU/driver/Proton e plataforma. Arquitetura de CPU não comprova arquitetura do jogo; artefato que exija x86/x86_64 específicos permanece em revisão enquanto a arquitetura do jogo não puder ser confirmada. Ausência de requisito específico não é inventada como incompatibilidade.

URLs de assinatura podem ser identificadas, mas **verificação criptográfica de assinatura não foi implementada**. DLLs são código executável no contexto do jogo; mesmo com checksum publicado, continuam bloqueadas nesta fase.

## Pipeline e limites

1. Resolver instalação pela descoberta Steam existente, incluindo bibliotecas em outros discos.
2. Gerar prévia somente de leitura: AppID, ambiente e limitações, publicador/versão, artefatos/tamanho/hashes, destinos, originais, requisitos, riscos, backup e rollback.
3. Obter confirmação de aplicação e consentimento explícito para o download na interface.
4. Revalidar fingerprint, raiz e diretórios, jogo e Steam fechados.
5. Consultar publicação atual e baixar todos os artefatos para preparação privada; verificar todos antes de iniciar escrita.
6. Validar formatos, membros e conteúdo; guardar backups verificados de todos os destinos.
7. Persistir a intenção de escrita (`writing` e hash esperado) no journal antes de cada substituição.
8. Instalar, verificar no disco e persistir histórico; reverter passos escritos se uma etapa posterior falhar.

| Limite | Valor |
| --- | --- |
| Payload por artefato | 8 MiB |
| Metadados consultados na preparação | 1 MiB |
| Configuração individual | 256 KiB |
| Arquivos selecionados por artefato | 8 |
| Entradas ZIP, incluindo não selecionadas | 128 |
| Expansão declarada total do ZIP | 2 MiB |
| Razão máxima de expansão por entrada | 100× |
| Redirecionamentos | 4 |
| Timeout de conexão / requisição | 6 s / 20 s |

O cliente só aceita HTTPS com TLS normal para `api.github.com`, `github.com`, `release-assets.githubusercontent.com` e `objects.githubusercontent.com`, inclusive no redirecionamento. Credenciais na URL, portas não padrão e esquemas locais são recusados. O payload inicial deve ser uma URL canônica de asset do repositório reconhecido. Leituras têm limite independentemente de `Content-Length`.

Cancelamento pertence à operação em staging, é conferido entre leituras e antes de iniciar a fase de escrita. Uma leitura bloqueada pode esperar o timeout de rede; não se promete interrupção instantânea. Após começar a escrever, o journal e a recuperação completam ou desfazem a transação em vez de abandoná-la no meio.

ZIPs malformados, CRC inválido, duplicação de nomes, nomes absolutos, `..`, caminhos Windows, symlinks e entradas especiais são recusados. A política examina também entradas não selecionadas e limita expansão antes de ler seu conteúdo. Os arquivos comprimidos nunca escrevem diretamente na instalação.

## Confinamento e concorrência

A raiz autorizada é a pasta do jogo detectado, sob `steamapps/common`. O executor percorre diretórios com descritores `openat`, `O_DIRECTORY` e `O_NOFOLLOW`; preserva identidade de dispositivo/inode da raiz e do diretório pai da prévia. Diretórios de destino precisam existir nesta fase. Caminhos absolutos, traversal, symlinks, arquivos especiais, hard links e destinos em outro jogo são recusados.

A escrita usa temporário exclusivo no diretório fixado, `fsync`, permissões sem bits executáveis e `renameat2`. Arquivo novo usa `RENAME_NOREPLACE`. Substituição usa `RENAME_EXCHANGE`, conservando a entrada deslocada até conferir que corresponde ao original com backup. Se uma alteração externa entrar na janela da substituição, o executor tenta restaurá-la; se houver nova concorrência, mantém os arquivos e exige recuperação, com a localização do temporário na mensagem.

A remoção no rollback também conserva uma entrada em quarentena até conferir seu hash. Nenhuma limpeza deve descartar conteúdo externo divergente. Arquivos somente leitura são recusados. Steam/jogo são verificados novamente após staging e antes de cada gravação; FRX não fecha processos.

Há atomicidade por troca de arquivo em sistemas que suportem essas operações Linux. **Não existe garantia de atomicidade global de vários arquivos**, exclusão total de escritores não cooperantes com descritores já abertos, ou proteção contra um atacante que controle o próprio diretório de dados do usuário. Falhas, reinícios e divergências são tratados pelo journal e backup; sistemas sem as operações necessárias falham sem usar fallback inseguro. Não se usa comparação de SHA do `localconfig.vdf` inteiro como substituto da validação seletiva de LaunchOptions.

## Histórico, rollback e atualizações Steam

Backups, journal e staging pertencem ao diretório privado `fix-engine/` do FRX. Staging da operação é removido ao terminar, cancelar ou recuperar uma operação interrompida; backups e histórico persistem. A intenção de escrita durável permite reconhecer um reinício entre a troca do arquivo e a confirmação final.

Rollback tem prévia e confirmação. Restaura o original cujo backup confere ou remove somente o arquivo adicionado e ainda correspondente ao hash instalado. Conteúdo externo divergente bloqueia a restauração automática. Ausência do patch não é considerada restauração se o original também não corresponder. Recuperação parcial permanece sinalizada. Quando conteúdo externo é retido em um temporário por concorrência, `manualRecoveryRequired` persiste no journal: conferir a presença do patch ou iniciar outro rollback não encerra falsamente essa pendência. A mensagem conserva a localização para revisão manual; não existe liberação automática desse caso nesta fase. Uma tentativa de rollback interrompida usa novos nomes de backup ao reiniciar e conserva as cópias anteriores.

Após atualização/verificação Steam, os hashes e identidades no disco podem mudar. O FRX mostra divergência, evita promover uma configuração não verificada e não reaplica automaticamente. Mudança de build conhecido, fonte, versão ou hash exige revalidação. Não há watcher permanente ou prova de que um processo externo nunca escreverá; as conferências ocorrem ao analisar, pré-visualizar, aplicar, verificar e capturar.

## Smart Packs e Performance Lab

O Builder valida integralmente cada receita e a combinação: AppID, conflitos de variáveis e de arquivo, interseção dos requisitos e política de artefatos. Uma etapa sem suporte impede usar a receita completa; o pacote não aproveita apenas sua variável ignorando o arquivo obrigatório.

Composições mistas usam o mesmo plano, journal, backup e rollback do Fix Engine. Todos os downloads são preparados antes de editar LaunchOptions ou arquivos. A identidade acrescenta destinos, hashes de conteúdo, IDs/versões/hashes dos artefatos e versão do formato. Pacote apenas de arquivos não é baseline.

A captura registra arquivos efetivamente instalados e seus hashes, junto de receitas/fingerprints e contexto do ambiente. Durante staging, um hash planejado pode servir para exibir progresso, mas não vira benchmark verificado. Mudança de artefato, configuração, AppID, GPU, driver ou Proton impede reutilizar evidência incompatível para a recomendação atual. Download concluído não é evidência de FPS.

## Compatibilidade de dados

Os novos campos são aditivos e têm defaults. Operações, sessões e packs anteriores continuam carregáveis sem inventar identidade de GPU ou metadados de arquivos. IDs legados dos packs apenas de variáveis são preservados. Nenhum benchmark, log, backup ou histórico antigo é apagado.

Gerador `1.3.0`, regras `2026.10.27-4`, compositor/ranking `1.4.0`: os caches são revalidados pela versão e pelo contexto, preservando registros históricos. Driver, inventário/seleção de GPU e Proton integram o fingerprint do cache de receitas. Dados sem contexto suficiente não recebem atributos do jogo aberto silenciosamente. Ajustes tipados de `dxvk.conf`, soluções compostas e validação de aplicabilidade estão descritos em [ACTIONABLE-FIXES.md](ACTIONABLE-FIXES.md).

## Como testar com segurança

Os testes Rust usam `tempfile`, instalações e processos fictícios; os testes React usam `mockIPC`. Fixtures não são instaladas no modo real nem importadas como benchmarks.

```bash
npm test
npm run typecheck
npm run build:web
(cd src-tauri && cargo fmt --check)
(cd src-tauri && cargo clippy --all-targets -- -D warnings)
(cd src-tauri && cargo test)
# Integração optativa, somente TLS local controlado; requer openssl e python3:
(cd src-tauri && cargo test controlled_https_download_install_restart_and_rollback --lib -- --ignored --nocapture)
npm run build
```

O teste TLS gera CA e certificado temporários, valida cadeia/hostname, recusa CA desconhecida e nome errado, e baixa metadados/payload sintéticos por HTTPS localhost. O transporte de teste traduz as URLs canônicas fictícias para esse servidor, sem desabilitar verificação TLS nem alterar a política de produção. Ele percorre prévia, download, hash, staging, backup, instalação, histórico/reinício e rollback/reinício no Engine existente. Não é uma instalação de um asset real no GitHub.

A compilação exige Rust **1.88 ou superior** devido à biblioteca ZIP 9.1.1. O ambiente de validação usa Rust 1.97. Resultados finais e evidências visuais serão registrados em [VALIDACAO-HARDWARE-FILE-ENGINE.md](VALIDACAO-HARDWARE-FILE-ENGINE.md).
