sed -i 's/queue.write_buffer(page_fault_buffer, 0, bytemuck::bytes_of(&zero));/encoder.clear_buffer(page_fault_buffer, 0, Some(4));/' fluoderpod_render/src/virtual_geometry/mod.rs
sed -i 's/let zero: u32 = 0;//' fluoderpod_render/src/virtual_geometry/mod.rs
