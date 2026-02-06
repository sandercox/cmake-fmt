target_compile_options(foo PRIVATE $<$<CONFIG:Debug>:-g>)
target_compile_options(foo PRIVATE $<$<AND:$<BOOL:${VAR}>,$<CONFIG:Debug>>:-g>)
set(FLAGS $<$<AND:$<BOOL:${VAR}>,$<OR:$<CONFIG:Debug>,$<CONFIG:RelWithDebInfo>>>:-g>)
target_compile_definitions(foo PRIVATE $<$<BOOL:${USE_FEATURE}>:FEATURE_ENABLED>)
