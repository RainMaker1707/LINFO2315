idf_component_register(
    SRCS "main.c"
    INCLUDE_DIRS "../ffi/"
)

add_prebuilt_library(libffi "../ffi/target/xtensa-esp32s3-espidf/release/libffi.a")
target_link_libraries(${COMPONENT_LIB} PRIVATE libffi -Wl,--allow-multiple-definition)
