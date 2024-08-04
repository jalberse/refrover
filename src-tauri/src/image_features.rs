use ndarray::Array1;
use uuid::Uuid;

use crate::preprocessing::FEATURE_VECTOR_LENGTH;


struct ImageFeatures
{
    id: Uuid,
    feature_vector: Array1<f32>,
}