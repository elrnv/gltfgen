use gltf::Gltf;

pub(crate) fn assert_eq_gltf_with_bytes(expected: &Gltf, actual: &Gltf) {
    assert_eq_gltf(expected, actual);
    assert_eq!(&expected.blob, &actual.blob);
}
pub(crate) fn assert_eq_gltf(expected: &Gltf, actual: &Gltf) {
    // Buffers
    assert_eq!(1, actual.buffers().count());
    let exp_buffer = expected.buffers().next().unwrap();
    let act_buffer = actual.buffers().next().unwrap();
    assert_eq!(exp_buffer.index(), act_buffer.index());
    assert_eq!(exp_buffer.length(), act_buffer.length());
    assert_eq!(exp_buffer.name(), act_buffer.name());

    // Buffer Views
    for (exp_view, act_view) in expected.views().zip(actual.views()) {
        assert_eq!(exp_view.index(), act_view.index());
        assert_eq!(exp_view.length(), act_view.length());
        assert_eq!(exp_view.offset(), act_view.offset());
        assert_eq!(exp_view.stride(), act_view.stride());
        assert_eq!(exp_view.name(), act_view.name());
        assert_eq!(exp_view.target(), act_view.target());
    }

    // Accessors
    for (exp_acc, act_acc) in expected.accessors().zip(actual.accessors()) {
        assert_eq!(exp_acc.index(), act_acc.index());
        assert_eq!(exp_acc.size(), act_acc.size());

        assert_eq!(exp_acc.offset(), act_acc.offset());
        assert_eq!(exp_acc.count(), act_acc.count());
        assert_eq!(exp_acc.data_type(), act_acc.data_type());
        assert_eq!(exp_acc.dimensions(), act_acc.dimensions());
        assert_eq!(exp_acc.min(), act_acc.min());
        assert_eq!(exp_acc.max(), act_acc.max());
        assert_eq!(exp_acc.name(), act_acc.name());
        assert_eq!(exp_acc.normalized(), act_acc.normalized());
        if let Some(exp_sparse) = exp_acc.sparse() {
            let act_sparse = act_acc.sparse().unwrap();
            assert_eq!(exp_sparse.count(), act_sparse.count());
            let exp_indices = exp_sparse.indices();
            let act_indices = act_sparse.indices();
            assert_eq!(exp_indices.offset(), act_indices.offset());

            assert_eq!(exp_sparse.values().offset(), act_sparse.values().offset());
        }
    }

    // Nodes
    for (exp_node, act_node) in expected.nodes().zip(actual.nodes()) {
        assert_eq!(exp_node.index(), act_node.index());
        let exp_mesh = exp_node.mesh().unwrap();
        let act_mesh = act_node.mesh().unwrap();
        assert_eq!(exp_mesh.index(), act_mesh.index());
        assert_eq!(exp_mesh.name(), act_mesh.name());
        assert_eq!(exp_mesh.weights(), act_mesh.weights());

        // Primitives
        for (exp_prim, act_prim) in exp_mesh.primitives().zip(act_mesh.primitives()) {
            assert_eq!(exp_prim.index(), act_prim.index());
            assert_eq!(exp_prim.bounding_box(), act_prim.bounding_box());

            // For some reason the attributes may not be ordered properly. In reality we only care
            // that they are the same, since the order doesn't matter.

            for exp_attrib in exp_prim.attributes() {
                let act_attrib = act_prim
                    .attributes()
                    .find(|a| a.1.index() == exp_attrib.1.index())
                    .unwrap();
                assert_eq!(exp_attrib.0, act_attrib.0);
                assert_eq!(exp_attrib.1.index(), act_attrib.1.index());
                assert_eq!(exp_attrib.1.size(), act_attrib.1.size());

                assert_eq!(exp_attrib.1.offset(), act_attrib.1.offset());
                assert_eq!(exp_attrib.1.count(), act_attrib.1.count());
                assert_eq!(exp_attrib.1.data_type(), act_attrib.1.data_type());
                assert_eq!(exp_attrib.1.dimensions(), act_attrib.1.dimensions());
                assert_eq!(exp_attrib.1.min(), act_attrib.1.min());
                assert_eq!(exp_attrib.1.max(), act_attrib.1.max());
                assert_eq!(exp_attrib.1.name(), act_attrib.1.name());
                assert_eq!(exp_attrib.1.normalized(), act_attrib.1.normalized());
                if let Some(exp_sparse) = exp_attrib.1.sparse() {
                    let act_sparse = act_attrib.1.sparse().unwrap();
                    assert_eq!(exp_sparse.count(), act_sparse.count());
                    let exp_indices = exp_sparse.indices();
                    let act_indices = act_sparse.indices();
                    assert_eq!(exp_indices.offset(), act_indices.offset());

                    assert_eq!(exp_sparse.values().offset(), act_sparse.values().offset());
                }
            }

            // Compare Material
            let exp_mat = exp_prim.material();
            let act_mat = act_prim.material();

            assert_eq!(exp_mat.index(), act_mat.index());
            assert_eq!(exp_mat.alpha_cutoff(), act_mat.alpha_cutoff());
            assert_eq!(exp_mat.alpha_mode(), act_mat.alpha_mode());
            assert_eq!(exp_mat.double_sided(), act_mat.double_sided());
            assert_eq!(exp_mat.name(), act_mat.name());
            // pbr_metallic_roughness
            assert_eq!(exp_mat.emissive_factor(), act_mat.emissive_factor());
        }
    }
}
