# AI Search — FRX

Módulo de pesquisa, ordenação e sumarização de correções para jogos Windows no Linux,
com contexto de hardware. Faz parte do FRX, mas não substitui o Fix Engine.

## O que é e o que não é

**Faz:**
- Formula queries de busca a partir de um sintoma + `SystemSnapshot` + `GameInfo`
- Consulta providers enumeráveis (allowlist fechada)
- Ranqueia candidatos por pertinência ao hardware/sistema do usuário
- Sumariza com citação literal, sem reescrever o fix
- Entrega candidatos ao Fix Engine via `prepare_fix_plan`

**Não faz:**
- Não diagnostica. Não interpreta procedimentos.
- Não infere causalidade nem associa ação a resultado.
- Não promete FPS.
- Não executa shell, sudo, Winetricks, scripts.
- Não toca em prefixos, drivers, Proton.
- Não baixa nem instala arquivos fora do Fix Engine.

O fundamento está no [RXAI](RXAI.md): D-002 (recuperação sobreviveu em três coortes),
D-008 (modalidade ainda confunde execução, sugestão e resultado), D-010 (11/70
candidatos completos limitaram o classificador no v6). A conclusão é que o
recuperador é promissor e o intérprete não é — o módulo de busca respeita essa
fronteira.

## Limitações conhecidas

- driver_proximity retorna null quando os dois lados são de vendors diferentes ou quando a versão do Mesa não pôde ser lida com confiança. null não penaliza. É tratado como 'sem informação', não como 'sem match'.
- A versão do Mesa vem só de leitura de arquivos: banco local do pacman, `/var/lib/dpkg/status` ou o nome de `libgallium-<versão>.so`. Fontes que discordam dão null. Em sistemas RPM, só o nome da biblioteca serve, e ele existe a partir do Mesa 24.2.
- A leitura diz o que está instalado no sistema. Um Steam em Flatpak usa o próprio Mesa, que o FRX não enxerga.
- Os pesos do score são provisórios.

## Arquitetura
