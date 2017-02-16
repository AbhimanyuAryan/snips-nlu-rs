use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::mem;

use ndarray::prelude::*;

use tensorflow::Graph;
use tensorflow::ImportGraphDefOptions;
use tensorflow::Session;
use tensorflow::SessionOptions;
use tensorflow::StepWithGraph;
use tensorflow::Tensor;

use pipeline::Probability;

pub trait CNN {
    fn run(&mut self, features: &Array2<f64>) -> Array2<f64>;
}

pub struct TensorflowCNN {
    session: Session,
    graph: Graph,
}

impl TensorflowCNN {
    pub fn new(model_path: &Path) -> TensorflowCNN {
        let mut graph = Graph::new();
        let mut proto = Vec::new();
        File::open(model_path).unwrap().read_to_end(&mut proto);

        graph.import_graph_def(&proto, &ImportGraphDefOptions::new());
        let session = Session::new(&SessionOptions::new(), &graph).unwrap();

        TensorflowCNN {
            session: session,
            graph: graph,
        }
    }
}

impl CNN for TensorflowCNN {
    fn run(&mut self, features: &Array2<f64>) -> Array2<Probability> {
        let transposed_array = features.t();
        let tokens_count = transposed_array.shape()[0];
        let features_count = transposed_array.shape()[1];

        let mut x: Tensor<f32> = Tensor::new(&[tokens_count as u64, features_count as u64]);
        for row in 0..tokens_count {
            for col in 0..features_count {
                x[row * features_count + col] = *transposed_array.get((row, col)).unwrap() as f32;
            }
        }

        let mut step = StepWithGraph::new();
        step.add_input(&self.graph.operation_by_name_required("input").unwrap(),
                       0,
                       &x);
        let res =
            step.request_output(&self.graph.operation_by_name_required("predictions").unwrap(),
                                0);

        self.session.run(&mut step);

        let tensor_res: Tensor<f32> = step.take_output(res).unwrap();

        let mut vec = Vec::with_capacity(tensor_res.data().len());
        vec.extend_from_slice(&tensor_res.data());
        let vec: Vec<Probability> = unsafe { mem::transmute(vec) };

        Array::from_vec(vec)
            .into_shape((tensor_res.dims()[0] as usize, tensor_res.dims()[1] as usize))
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use ndarray::prelude::*;

    use models::cnn::{CNN, TensorflowCNN};

    #[test]
    #[ignore]
    fn cnn_works() {
        let model_path = Path::new("../data/snips-sdk-models-protobuf/tokens_classification/cnn_model_quantized.pb");
        let mut cnn = TensorflowCNN::new(model_path);
        let features = arr2(&[[1.0], [2.0]]);

        let probabilities = cnn.run(&features);

        println!("probabilities: {}", probabilities);
    }
}
