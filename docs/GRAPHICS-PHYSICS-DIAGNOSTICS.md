# Diagnóstico gráfico e de física — Resolution Intelligence

Este documento acompanha o desenvolvimento da v0.5. A versão do aplicativo não indica que todas as tecnologias abaixo sejam automaticamente corrigíveis. A pesquisa é somente leitura; nenhum dos procedimentos descritos aqui foi aplicado à Steam ou ao Batman real.

## Evidência antes de conclusão

O perfil técnico distingue hardware compatível, implementação no jogo, disponibilidade no ambiente, configuração solicitada e observação em execução. Uma biblioteca encontrada por inspeção estática é um indício de componente; não comprova recurso ativo. Uma opção de inicialização é configuração solicitada; não comprova funcionamento. FPS do MangoHud, sozinho, não confirma geração de quadros nem mede latência.

O Research Engine consulta documentação pública dos mantenedores e a API pública do GitHub, usando o cliente HTTP existente. A seleção depende do sintoma e da ferramenta Proton identificada. O fabricante da GPU só restringe a busca quando a GPU utilizada foi observada; inventário e sistemas com seleção desconhecida não impõem essa restrição. No máximo seis documentos, uma release e uma busca de issues por consulta; respostas de documentos limitadas a 256 KiB e respostas JSON a 512 KiB. O cliente existente impõe HTTPS, hosts e redirecionamentos permitidos, timeout, ETag, cache e rate limits. Não há scraping de ProtonDB nem pressuposto de API autorizada para sites externos.

As fontes integradas incluem Proton, GE-Proton, CachyOS Proton, DXVK, VKD3D-Proton, DXVK-NVAPI, NVIDIA Streamline, AMD FidelityFX e nvidia-libs. São projetos técnicos, não um catálogo de receitas por jogo. A documentação técnica geral fica com `appId = null`. Issues só recebem AppID quando o próprio conteúdo apresenta identificador explícito ou URL da Steam e não contém identificação conflitante. O AppID na consulta, a semelhança de nomes e um número entre parênteses não são prova suficiente.

Cada registro conserva URL canônica, publicador, tipo, versão quando verificável, data de publicação e atualização quando fornecidas separadamente, data de consulta, limitações e estado de cache vencido. A identidade incorpora SHA-256 da URL e conteúdo; não equivale a uma assinatura de autenticidade. Conteúdo antigo continua sendo relato histórico. Uma issue aberta não demonstra que o bug existe na versão atual, e uma issue fechada não demonstra resolução no computador do usuário.

Texto externo é dado sem autoridade: não altera allowlists, não produz comandos de shell e não autoriza downloads. Citações e nomes de opções servem para explicar mecanismos e requisitos. As receitas executáveis continuam pertencendo ao Fix Engine, com revalidação, prévia, consentimento, backup e rollback existentes.

## Capacidades e resolvers

| Área | O que pode ser investigado | O que não se deve concluir automaticamente |
| --- | --- | --- |
| Pilha gráfica | Componentes DXVK, VKD3D, WineD3D e NVAPI; logs autorizados; configuração conhecida | DirectX 12 não implica ray tracing; inventário não prova GPU ativa |
| PhysX | Componentes legados, documentação de bibliotecas NVIDIA, limitações de Proton e relatos do mesmo AppID | GPU NVIDIA moderna não garante PhysX por GPU funcionando |
| DLSS | Indícios do jogo, GPU, NVAPI/NGX, versão do Proton e mecanismo documentado | Uma DLL não adiciona implementação de DLSS a qualquer jogo |
| Frame generation | Tecnologia específica, requisitos, configuração e possíveis evidências runtime | FPS médio não distingue frames renderizados, gerados e apresentados; latência fica desconhecida sem instrumentação |
| Ray tracing | Requisitos DXR/Vulkan/VKD3D e configuração/logs, quando disponíveis | Recurso desativado não significa quebrado; ativá-lo não é otimização universal |
| FSR | Implementação específica, versão, hardware e mecanismo da ferramenta selecionada | Nem toda versão exige AMD; suporte de uma implementação não vale para todas |
| XeSS | Indícios do componente e documentação pertinente | Presença de biblioteca não prova ativação nem compatibilidade completa |

As evidências comunitárias não se tornam benchmarks. A comparação de desempenho continua no Performance Lab, restrita ao mesmo AppID, GPU observada, driver, Proton, pacote/fingerprint e condições comparáveis. Resultados de um jogo ou hardware não promovem automaticamente outro.

## GE-Proton: mecanismo documentado não é aplicação genérica

Consulta primária em **27/10/2026**: a [release GE-Proton11-11](https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/GE-Proton11-11) foi publicada em 19/10/2026. O [README dessa tag exata](https://github.com/GloriousEggroll/proton-ge-custom/blob/GE-Proton11-11/README.md) documenta `PROTON_DLSS_UPGRADE` e `PROTON_FSR4_UPGRADE`. FSR4 exige uma implementação de jogo compatível com FSR3.1 e suporte adequado de GPU/driver. A antiga opção separada de RDNA3 é obsoleta nessa versão. A escolha do binário usa um manifesto externo e pode fazer fallback se a versão solicitada não estiver disponível.

Consequências para o FRX: consultar README master não comprova suporte de uma versão instalada; o motor tenta a tag exata quando identifica `GE-Proton<major>-<release>`. A atualização de biblioteca aciona trabalho adicional do Proton, com versões e efeitos que o rollback de uma variável não necessariamente desfaz. Portanto, essas opções não são promovidas à execução automática apenas por aparecerem na documentação. O diagnóstico explica o mecanismo e mantém revisão até haver política de execução e reversão para a classe específica. Proton Valve não herda opções exclusivas de GE.

## Frame generation: limites de hardware

A [documentação técnica NVIDIA sobre integração DLSS 4, publicada em 30/01/2025](https://developer.nvidia.com/blog/how-to-integrate-nvidia-dlss-4-into-your-game-with-nvidia-streamline/) distingue Frame Generation das séries RTX 40/50 e Multi Frame Generation da RTX 50. A [página oficial de capacidades DLSS](https://www.nvidia.com/en-us/geforce/technologies/dlss/) foi consultada em 27/10/2026 para conferir a distinção.

Uma RTX 4060 não recebe recomendação de Multi Frame Generation que exija RTX 50. Isso não determina se FG comum funciona num jogo particular: jogo, driver, Proton e instrumentação ainda precisam sustentar a conclusão. Não se oferece download para contornar um requisito físico.

## Batman Arkham City GOTY / NVIDIA / PhysX

O caso usa o AppID `200260`, com a edição confirmada pela [issue Proton #309](https://github.com/ValveSoftware/Proton/issues/309). A [issue GE-Proton #196](https://github.com/GloriousEggroll/proton-ge-custom/issues/196) é uma pista comunitária para problemas de PhysX e [SveSop/nvidia-libs](https://github.com/SveSop/nvidia-libs). Sem identificação explícita suficiente da edição/AppID, a pista não vira automaticamente uma receita específica ou medição do Batman GOTY.

O [README do Proton CachyOS](https://github.com/CachyOS/proton-cachyos/blob/cachyos_main/README.md), consultado em 27/10/2026, descreve `PROTON_NVIDIA_LIBS`, incompatibilidade com WoW64 e uma ressalva de bibliotecas de 32 bits em RTX 4000/5000. São requisitos contextuais; não provam que o caminho funcione no jogo do usuário. A documentação pode mudar e deve ser revalidada com a versão selecionada.

O projeto nvidia-libs tem releases verificáveis, mas envolve bibliotecas executáveis, arquivos TAR e procedimentos para prefixo/Proton. Essa classe continua **Requer revisão**. A descoberta não é reduzida a uma instrução inventada de copiar DLL. O arquivo instalado historicamente pelo usuário continua **não identificado**; os FPS informais relatados não entram no Performance Lab. Veja também [a investigação detalhada anterior](BATMAN-PHYSX-INVESTIGATION.md).

## Segurança e limites de suporte

O caminho aplicável da v0.5 reutiliza as operações já suportadas do Fix Engine. O Research Engine não amplia sozinho as permissões de arquivos, binários, variáveis ou seleção de Proton. Aplicar uma receita significa registrar o resultado da operação; a interface deve continuar exibindo “Correção aplicada — funcionamento ainda não verificado” até existir um método adequado.

O motor não coleta logs automaticamente, não inicia jogos, não altera drivers, não fecha a Steam, não usa sudo e não baixa binários durante a pesquisa. Os documentos e metadados recebidos podem orientar uma revisão, sem substituir licença, procedência, checksum, compatibilidade, destino e reversibilidade.

Limitações atuais: a pesquisa de issues é limitada e pode não encontrar relatos sem AppID explícito; discussões e comentários seguem a descoberta existente, sem nova interpretação irrestrita; metadados de hardware presentes em prosa não se tornam requisitos confiáveis automaticamente; runtime de PhysX/DLSS/FSR/FG pode permanecer desconhecido; ausência de solução segura é um resultado válido.

## Verificação automatizada

`cargo test --manifest-path src-tauri/Cargo.toml resolution::research::tests --lib` concluiu com **12 aprovados, zero falhas**, em 27/10/2026. As fixtures são explicitamente sintéticas e cobrem escopo do AppID, fontes ambíguas, URLs hostis, conteúdo sem autoridade, documentação geral, tags exatas do GE, publicação versus atualização, metadados de release sem baixar assets, cache/offline/rate limit/ETag, reinício, isolamento entre jogos, GPU observada versus inventário, limites de resposta, JSON malformado e identidade por conteúdo. GETs públicos somente leitura também confirmaram resposta 200 dos nove caminhos de documentação integrados; o AMD usa `readme.md` minúsculo. Nenhum payload binário foi baixado.

Esses testes não representam instalação de patch nem benchmark real. Os resultados consolidados ficam em [VALIDACAO-v0.5.md](VALIDACAO-v0.5.md).
