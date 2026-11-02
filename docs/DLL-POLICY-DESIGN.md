# Política de DLL e TAR

**Estado:** implementado em 2026-11-02, só com testes de bytes sintéticos. Nenhuma DLL real foi instalada, e o fluxo não foi aberto na janela do app.

**Prioridade do FRX:** variáveis de inicialização da Steam. DLL é uma exceção, para casos como um arquivo publicado que vai na pasta do jogo.

## Escopo de escrita

Só a pasta do jogo, em `steamapps/common/<jogo>`. O `confined.rs` não foi expandido para o prefixo, e a recusa usa sempre a mesma mensagem, `PREFIX_REFUSED`: "O FRX não altera prefixos do Proton (compatdata/ ou pfx/)…".

| Tentativa | Resultado |
|---|---|
| Raiz em `compatdata/` ou `pfx/` | Recusada por `check_game_root` |
| Caminho relativo com componente `compatdata` ou `pfx`, em qualquer caixa | Recusado por `Target::open` e `Target::open_binary`, e antes disso por `artifacts::validate` |
| `..` no caminho | Recusado por `check_rel_path` e `check_dll_rel_path` |
| Symlink na pasta do jogo apontando para o prefixo | Recusado por `O_NOFOLLOW` |

**Consequência:** uma correção que dependa de prefixo, registro do Wine, override de DLL ou alteração da instalação do Proton continua em revisão. O `nvidia-libs` documenta esses procedimentos, então ele não fica instalável.

## De onde vem uma DLL

`ArtifactCandidate.dlls` só é preenchido pelo provider do GitHub (`fixes/providers/artifacts.rs`), e só a partir das instruções do publicador numa release de um repositório de `recognized_repository`:

- `Install: Binaries/x.dll <- x.dll`, quando o asset é a própria DLL. O hash é o `digest` da API da release.
- `Extract: pacote.zip::x64/x.dll -> Binaries/x.dll` ou `pacote.tar::…`, com `SHA256 (x64/x.dll)=…` para o membro.
- O bloco `frx-file-fix`, com o campo `dlls`.

Comentário de issue nunca escolhe destino nem hash. Sem hash verificável, a descoberta fica `Partial`, e o núcleo não gera receita.

**Mudança em relação à versão anterior deste documento:** ela dizia que receitas geradas a partir de descobertas nunca instalariam DLL. Agora uma DLL de publicador reconhecido, com instruções completas e `digest`, vira receita gerada. A proteção é o consentimento explícito, verificado no backend.

## Segurança

- **O hash prova só que o arquivo é o da release.** Ele não prova que o arquivo é benigno, compatível ou mais rápido. Não há Authenticode nem X.509.
- **Conteúdo:** `validate_dll` exige um cabeçalho PE (`MZ` e `PE\0\0`) e no máximo 64 MiB (`MAX_BINARY`). Isso só evita gravar algo que nem é DLL.
- **Consentimento:** `DllArtifact.requires_explicit_user_consent` tem default `true`, e `validate` recusa `false`. `FixPlan.requires_explicit_user_consent` fica `true` quando algum passo grava DLL. `FixEngine::apply_with_consent` recusa sem `binary_consent` igual ao `fingerprint` da prévia, antes de qualquer download, e a recusa não consome a prévia. O comando `apply_fix_plan` recebe esse valor em `binaryConsent`.
- **Smart Pack:** uma DLL não entra em pacote, porque exige confirmação própria.
- **Interface:** o cartão do Diagnóstico (`Suggestion.thirdPartyBinary`), a prévia e a confirmação mostram o aviso vermelho "Instalação de binário de terceiros. O FRX não se responsabiliza pelo conteúdo.". O botão de instalar só habilita com a caixa da DLL marcada.

## Tipos

| Campo de `DllArtifact` | Regra |
|---|---|
| `sourcePath` | Membro exato do pacote, ou `null` se o download for a própria DLL |
| `destinationRelativePath` | Relativo à raiz do jogo, termina em `.dll`, validado na escrita |
| `expectedSha256` | Obrigatório, sem default |
| `expectedOriginalSha256` | Hash da DLL substituída. `null` só quando o destino não existe; com destino existente e diferente, o plano para |
| `requiresExplicitUserConsent` | Default `true`; `false` é recusado |

O tipo usa `deny_unknown_fields`, então um manifesto com campo desconhecido falha.

## TAR

`ArtifactFormat::Tar` aceita só `.tar` sem compressão, lido em memória por `tar_entries`. `.tar.xz` e `.tar.gz` continuam `Unknown`, porque exigem dependências novas ainda não revisadas.

- **Tipos de entrada:** só arquivo comum e diretório. Link simbólico, hard link, dispositivo, FIFO, PAX global e entrada esparsa recusam o pacote inteiro.
- **Nomes:** prefixo ustar, nome longo GNU (`L`) e `path` PAX são resolvidos antes de `safe_member`. Um `./` inicial é removido. Nome duplicado, com qualquer caixa, recusa o pacote.
- **PAX:** chaves que mudam tamanho ou link, como `size` e `linkpath`, recusam o pacote.
- **Integridade:** checksum de cada cabeçalho, tamanho octal e dois blocos zerados no fim. Arquivo truncado é recusado.
- **Extração:** só os membros selecionados, cada um conferido contra o próprio SHA-256. Nada é extraído para disco antes do backup.

## Rollback

Reaproveita a troca com `RENAME_EXCHANGE` de `confined.rs`. A DLL é gravada por um `Target` aberto com `open_binary`, que só muda o limite de tamanho.

**Aplicar:** baixar para staging privado, conferir o SHA-256 do download e do membro, conferir que o jogo está fechado e que o destino tem o hash da prévia, guardar o backup fora da pasta do jogo, trocar de forma atômica e reler o destino.

**Reverter:** se o destino tem o hash instalado, o backup é restaurado ou, se não existia original, o arquivo é removido. Se o destino mudou, nada é sobrescrito e a operação fica em conflito.

**Diário:** `Step::GameFile.dll` guarda só os metadados e o hash. Os bytes verificados ficam em memória entre o staging e a escrita, e não vão para o JSON. `JOURNAL_SCHEMA` continua 1, porque o campo é opcional.

**Limite aceito:** `game_running` procura `SteamAppId` em `/proc` e devolve `false` se não conseguir ler. Um processo que sobreviva sem essa variável pode manter a DLL antiga mapeada, e esse processo continua com a versão anterior até sair.

## Removido desta versão

| Item | Motivo | Quando reabrir |
|---|---|---|
| Authenticode e `publisherAllowlist` | Não há crate estável que verifique por completo, e o escopo ficou na pasta do jogo | Se o FRX quiser aceitar DLLs sem revisão por hash |
| Escrita no prefixo | Afeta todo o ambiente do jogo, e `dosdevices/z:` aponta para `/` | Se um caso real exigir, com novo design |
| `.tar.xz` | Dependências de TAR e XZ sem revisão de licença e superfície | Depois dessa revisão |

## Testes

Todos com bytes sintéticos, sem rede e sem DLL real:

- `fix_engine/artifact_tests.rs`: instalação e rollback de DLL nova e substituída, após reinício; recusa sem consentimento e com fingerprint errado; download adulterado e original desconhecido; destino em `compatdata/` e `pfx/`; extração seletiva de TAR com rollback; TAR com link, hard link, dispositivo, FIFO, traversal, caminho absoluto, duplicata, PAX global, `linkpath`, `path` com traversal, checksum errado e truncamento.
- `fix_engine/confined.rs`: escrita de texto e de DLL em `compatdata/` ou `pfx/` falha com `PREFIX_REFUSED`.
- `fixes/providers/artifacts.rs`: DLL e membro de TAR documentados pelo publicador; `.exe` e `.tar.xz` ficam em revisão.
- Interface: aviso no cartão e caixa de consentimento com `binaryConsent` (`src/test/diagnosis.test.tsx` e `src/test/fileFix.test.tsx`).

## Decisão pendente

**Redistribuição:** o FRX nunca espelha binários. Ele baixa da origem, e a licença informada aparece na prévia, mas não é verificada.
