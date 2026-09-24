#![allow(dead_code)]
#[path = "../src/admission_evidence.rs"]
mod admission_evidence;
#[path = "../src/native_admission_source/mod.rs"]
mod native_admission_source;
use admission_evidence::{Failure, Request, Response};
use native_admission_source::{
    TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls::{BoundedHandshakeIo, exchange as bounded_exchange},
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
// Public test-only CA and identities, generated solely for loopback interoperability.
// These keys are fixtures and must never be installed in a deployed trust store.
const CA: &[u8] = &[
    48, 130, 2, 220, 48, 130, 1, 196, 160, 3, 2, 1, 2, 2, 20, 53, 150, 161, 237, 171, 190, 123,
    228, 180, 200, 111, 134, 70, 10, 244, 71, 133, 85, 90, 152, 48, 13, 6, 9, 42, 134, 72, 134,
    247, 13, 1, 1, 11, 5, 0, 48, 28, 49, 26, 48, 24, 6, 3, 85, 4, 3, 12, 17, 79, 116, 101, 114,
    121, 110, 32, 83, 49, 32, 84, 101, 115, 116, 32, 67, 65, 48, 30, 23, 13, 50, 53, 48, 49, 48,
    49, 48, 48, 48, 48, 48, 48, 90, 23, 13, 51, 53, 48, 49, 48, 49, 48, 48, 48, 48, 48, 48, 90, 48,
    28, 49, 26, 48, 24, 6, 3, 85, 4, 3, 12, 17, 79, 116, 101, 114, 121, 110, 32, 83, 49, 32, 84,
    101, 115, 116, 32, 67, 65, 48, 130, 1, 34, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5,
    0, 3, 130, 1, 15, 0, 48, 130, 1, 10, 2, 130, 1, 1, 0, 162, 202, 23, 154, 224, 96, 190, 4, 36,
    81, 99, 176, 89, 23, 188, 127, 80, 129, 184, 64, 237, 56, 47, 223, 176, 110, 245, 128, 39, 218,
    61, 59, 79, 105, 53, 154, 246, 164, 6, 131, 38, 100, 252, 131, 180, 101, 10, 157, 197, 191, 93,
    142, 27, 250, 122, 180, 214, 146, 66, 70, 10, 70, 126, 247, 41, 102, 63, 50, 96, 247, 24, 137,
    182, 3, 55, 146, 118, 110, 125, 126, 67, 90, 164, 102, 23, 95, 208, 122, 92, 69, 35, 108, 41,
    50, 249, 216, 14, 189, 128, 245, 255, 151, 42, 211, 216, 85, 196, 252, 154, 72, 241, 19, 99,
    76, 49, 88, 218, 87, 11, 108, 17, 131, 69, 199, 13, 44, 25, 109, 135, 24, 4, 125, 194, 229,
    148, 25, 49, 48, 116, 70, 48, 137, 168, 239, 207, 147, 61, 225, 87, 128, 1, 212, 173, 175, 241,
    72, 133, 132, 80, 131, 137, 187, 37, 79, 200, 197, 167, 94, 28, 86, 208, 54, 134, 246, 137,
    212, 201, 85, 21, 80, 213, 43, 68, 144, 93, 151, 108, 235, 85, 131, 132, 143, 21, 29, 35, 203,
    229, 43, 135, 29, 31, 43, 222, 88, 136, 177, 113, 197, 157, 55, 250, 48, 61, 226, 101, 198,
    227, 19, 246, 65, 160, 196, 198, 199, 131, 127, 66, 241, 183, 180, 15, 52, 48, 97, 18, 242, 72,
    40, 251, 210, 216, 52, 75, 39, 10, 122, 171, 209, 149, 87, 73, 150, 4, 59, 62, 189, 2, 3, 1, 0,
    1, 163, 22, 48, 20, 48, 18, 6, 3, 85, 29, 19, 1, 1, 255, 4, 8, 48, 6, 1, 1, 255, 2, 1, 0, 48,
    13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 11, 5, 0, 3, 130, 1, 1, 0, 139, 16, 227, 236, 230,
    207, 105, 197, 177, 197, 141, 177, 243, 188, 47, 78, 167, 228, 225, 68, 94, 53, 164, 60, 228,
    216, 245, 85, 72, 100, 166, 16, 107, 179, 143, 14, 244, 55, 85, 83, 227, 238, 137, 116, 117,
    94, 158, 21, 79, 173, 132, 6, 82, 126, 104, 184, 172, 29, 224, 236, 219, 33, 2, 234, 215, 157,
    138, 249, 99, 24, 47, 31, 225, 167, 18, 41, 175, 76, 111, 137, 143, 46, 232, 44, 46, 123, 215,
    78, 255, 122, 108, 78, 2, 19, 13, 233, 123, 136, 90, 194, 179, 126, 208, 138, 89, 82, 112, 89,
    39, 193, 69, 144, 32, 175, 10, 77, 98, 177, 202, 21, 179, 114, 179, 116, 210, 25, 74, 67, 172,
    39, 78, 123, 33, 125, 208, 88, 17, 237, 17, 64, 73, 246, 140, 125, 78, 0, 74, 115, 179, 223,
    187, 61, 165, 249, 66, 254, 121, 200, 230, 145, 125, 113, 142, 85, 8, 194, 97, 251, 32, 217,
    229, 48, 213, 147, 35, 69, 217, 123, 45, 223, 21, 90, 24, 231, 14, 192, 74, 242, 39, 17, 4,
    154, 250, 186, 115, 43, 193, 46, 49, 143, 184, 186, 35, 249, 127, 54, 85, 110, 200, 198, 176,
    48, 96, 0, 167, 28, 233, 217, 106, 62, 244, 27, 135, 250, 175, 29, 208, 218, 154, 106, 226, 5,
    150, 218, 85, 220, 249, 24, 25, 242, 147, 245, 52, 250, 145, 193, 219, 253, 31, 111, 241, 32,
    69, 204, 236, 249,
];
const SERVER: &[u8] = &[
    48, 130, 2, 253, 48, 130, 1, 229, 160, 3, 2, 1, 2, 2, 20, 99, 94, 190, 224, 211, 169, 151, 119,
    30, 251, 198, 204, 168, 158, 250, 136, 8, 60, 67, 13, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13,
    1, 1, 11, 5, 0, 48, 28, 49, 26, 48, 24, 6, 3, 85, 4, 3, 12, 17, 79, 116, 101, 114, 121, 110,
    32, 83, 49, 32, 84, 101, 115, 116, 32, 67, 65, 48, 30, 23, 13, 50, 53, 48, 49, 48, 49, 48, 48,
    48, 48, 48, 48, 90, 23, 13, 51, 53, 48, 49, 48, 49, 48, 48, 48, 48, 48, 48, 90, 48, 22, 49, 20,
    48, 18, 6, 3, 85, 4, 3, 12, 11, 115, 111, 117, 114, 99, 101, 46, 116, 101, 115, 116, 48, 130,
    1, 34, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 3, 130, 1, 15, 0, 48, 130, 1,
    10, 2, 130, 1, 1, 0, 152, 233, 65, 6, 10, 98, 212, 64, 133, 125, 106, 180, 126, 249, 31, 66,
    170, 78, 51, 127, 32, 173, 191, 65, 96, 170, 82, 149, 156, 150, 207, 244, 194, 31, 127, 114,
    90, 194, 163, 10, 0, 9, 191, 204, 81, 146, 136, 64, 15, 194, 232, 176, 64, 116, 40, 73, 29,
    154, 188, 94, 148, 151, 64, 136, 9, 91, 58, 50, 175, 208, 129, 48, 229, 90, 41, 249, 38, 156,
    3, 234, 67, 208, 60, 193, 126, 50, 93, 73, 56, 152, 93, 214, 220, 7, 125, 243, 92, 170, 103,
    221, 141, 28, 20, 214, 58, 52, 116, 252, 51, 113, 137, 120, 199, 170, 22, 221, 12, 80, 169,
    202, 37, 185, 123, 14, 23, 210, 213, 82, 5, 219, 181, 171, 212, 233, 230, 254, 21, 136, 135,
    90, 72, 227, 222, 215, 4, 34, 238, 58, 208, 54, 59, 116, 182, 199, 144, 99, 9, 137, 245, 73,
    234, 195, 128, 226, 10, 85, 188, 219, 44, 5, 217, 21, 115, 69, 89, 129, 84, 76, 133, 43, 161,
    55, 239, 52, 8, 145, 114, 116, 129, 71, 198, 118, 35, 85, 120, 94, 160, 242, 113, 64, 24, 52,
    86, 153, 198, 2, 80, 250, 88, 123, 30, 215, 174, 55, 66, 14, 80, 15, 137, 162, 47, 128, 167,
    179, 229, 64, 181, 193, 160, 144, 174, 128, 61, 82, 238, 58, 45, 74, 219, 127, 214, 59, 167,
    199, 246, 136, 11, 76, 243, 155, 36, 208, 241, 113, 111, 147, 2, 3, 1, 0, 1, 163, 61, 48, 59,
    48, 12, 6, 3, 85, 29, 19, 1, 1, 255, 4, 2, 48, 0, 48, 19, 6, 3, 85, 29, 37, 4, 12, 48, 10, 6,
    8, 43, 6, 1, 5, 5, 7, 3, 1, 48, 22, 6, 3, 85, 29, 17, 4, 15, 48, 13, 130, 11, 115, 111, 117,
    114, 99, 101, 46, 116, 101, 115, 116, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 11, 5, 0,
    3, 130, 1, 1, 0, 12, 63, 151, 183, 30, 119, 138, 40, 144, 84, 174, 58, 58, 53, 88, 37, 210, 88,
    0, 24, 121, 213, 28, 109, 97, 91, 99, 174, 51, 10, 225, 22, 135, 51, 229, 25, 186, 244, 22,
    205, 22, 228, 238, 245, 0, 67, 255, 144, 27, 53, 200, 155, 221, 135, 177, 146, 115, 98, 56,
    247, 130, 228, 96, 151, 150, 188, 192, 28, 111, 105, 39, 238, 195, 162, 189, 195, 41, 35, 121,
    7, 194, 35, 227, 187, 19, 179, 44, 119, 150, 165, 89, 215, 30, 33, 18, 107, 131, 219, 98, 222,
    67, 182, 78, 168, 82, 132, 20, 55, 31, 189, 221, 124, 35, 182, 221, 103, 4, 155, 3, 114, 148,
    52, 4, 124, 183, 9, 176, 254, 16, 158, 51, 8, 69, 66, 203, 139, 225, 102, 127, 150, 108, 25,
    157, 139, 23, 46, 140, 24, 92, 225, 195, 149, 88, 177, 161, 188, 208, 250, 254, 13, 17, 137,
    20, 186, 44, 160, 52, 154, 226, 250, 109, 140, 219, 228, 147, 65, 175, 252, 105, 220, 248, 141,
    111, 169, 166, 113, 228, 99, 103, 142, 110, 91, 54, 121, 63, 237, 70, 223, 160, 39, 214, 93, 1,
    80, 95, 106, 212, 149, 139, 101, 39, 242, 211, 33, 102, 232, 222, 145, 211, 175, 69, 208, 71,
    170, 98, 207, 223, 166, 175, 53, 109, 12, 50, 71, 226, 7, 161, 221, 35, 186, 225, 145, 176,
    228, 208, 199, 43, 147, 246, 250, 93, 194, 234, 222, 145, 221,
];
const SERVER_KEY: &[u8] = &[
    48, 130, 4, 189, 2, 1, 0, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 4, 130, 4,
    167, 48, 130, 4, 163, 2, 1, 0, 2, 130, 1, 1, 0, 152, 233, 65, 6, 10, 98, 212, 64, 133, 125,
    106, 180, 126, 249, 31, 66, 170, 78, 51, 127, 32, 173, 191, 65, 96, 170, 82, 149, 156, 150,
    207, 244, 194, 31, 127, 114, 90, 194, 163, 10, 0, 9, 191, 204, 81, 146, 136, 64, 15, 194, 232,
    176, 64, 116, 40, 73, 29, 154, 188, 94, 148, 151, 64, 136, 9, 91, 58, 50, 175, 208, 129, 48,
    229, 90, 41, 249, 38, 156, 3, 234, 67, 208, 60, 193, 126, 50, 93, 73, 56, 152, 93, 214, 220, 7,
    125, 243, 92, 170, 103, 221, 141, 28, 20, 214, 58, 52, 116, 252, 51, 113, 137, 120, 199, 170,
    22, 221, 12, 80, 169, 202, 37, 185, 123, 14, 23, 210, 213, 82, 5, 219, 181, 171, 212, 233, 230,
    254, 21, 136, 135, 90, 72, 227, 222, 215, 4, 34, 238, 58, 208, 54, 59, 116, 182, 199, 144, 99,
    9, 137, 245, 73, 234, 195, 128, 226, 10, 85, 188, 219, 44, 5, 217, 21, 115, 69, 89, 129, 84,
    76, 133, 43, 161, 55, 239, 52, 8, 145, 114, 116, 129, 71, 198, 118, 35, 85, 120, 94, 160, 242,
    113, 64, 24, 52, 86, 153, 198, 2, 80, 250, 88, 123, 30, 215, 174, 55, 66, 14, 80, 15, 137, 162,
    47, 128, 167, 179, 229, 64, 181, 193, 160, 144, 174, 128, 61, 82, 238, 58, 45, 74, 219, 127,
    214, 59, 167, 199, 246, 136, 11, 76, 243, 155, 36, 208, 241, 113, 111, 147, 2, 3, 1, 0, 1, 2,
    130, 1, 0, 40, 118, 20, 56, 34, 120, 107, 60, 149, 118, 13, 205, 172, 9, 132, 141, 145, 221,
    246, 127, 109, 168, 188, 2, 115, 47, 46, 130, 27, 56, 198, 215, 55, 164, 185, 7, 11, 224, 19,
    223, 58, 10, 90, 95, 152, 184, 34, 232, 124, 163, 49, 10, 46, 75, 238, 59, 6, 163, 226, 179,
    78, 51, 69, 166, 153, 127, 51, 195, 164, 197, 17, 226, 91, 214, 3, 100, 67, 164, 165, 222, 143,
    202, 34, 193, 122, 128, 158, 49, 153, 81, 172, 7, 6, 26, 207, 103, 157, 126, 19, 212, 202, 153,
    131, 35, 250, 171, 213, 198, 196, 96, 7, 37, 216, 3, 120, 96, 78, 121, 107, 172, 43, 86, 76,
    170, 250, 127, 213, 229, 32, 190, 64, 90, 177, 106, 213, 61, 96, 57, 17, 251, 161, 183, 21,
    239, 171, 127, 250, 176, 8, 187, 163, 230, 18, 186, 52, 249, 36, 202, 114, 88, 221, 230, 73, 2,
    105, 255, 119, 95, 140, 189, 148, 75, 51, 246, 187, 198, 197, 100, 212, 3, 98, 29, 17, 103,
    136, 91, 147, 72, 170, 24, 93, 142, 170, 29, 19, 115, 218, 239, 248, 119, 204, 120, 254, 107,
    40, 238, 183, 78, 18, 212, 37, 180, 3, 237, 76, 216, 90, 253, 102, 48, 132, 171, 28, 148, 149,
    108, 88, 101, 99, 179, 83, 227, 233, 124, 28, 117, 103, 172, 184, 57, 224, 105, 90, 238, 101,
    174, 9, 24, 170, 162, 98, 67, 75, 188, 229, 169, 2, 129, 129, 0, 216, 3, 70, 209, 20, 186, 180,
    254, 67, 138, 214, 75, 127, 165, 74, 217, 230, 20, 204, 187, 105, 227, 135, 1, 99, 150, 59,
    217, 155, 0, 64, 71, 126, 88, 21, 100, 125, 222, 229, 117, 94, 95, 28, 196, 246, 176, 152, 104,
    178, 183, 65, 241, 98, 228, 41, 47, 40, 91, 176, 52, 93, 32, 25, 231, 219, 212, 46, 97, 213,
    34, 145, 111, 183, 138, 181, 63, 30, 85, 247, 225, 234, 170, 31, 42, 115, 250, 233, 130, 94,
    19, 140, 221, 212, 156, 44, 35, 14, 20, 228, 57, 152, 166, 88, 225, 238, 41, 160, 167, 146,
    219, 160, 218, 37, 105, 63, 242, 8, 245, 146, 66, 60, 226, 50, 15, 191, 51, 206, 25, 2, 129,
    129, 0, 181, 55, 160, 73, 107, 99, 179, 208, 207, 115, 225, 251, 253, 165, 182, 101, 85, 39,
    55, 113, 11, 104, 50, 98, 98, 38, 110, 197, 136, 81, 163, 78, 12, 8, 210, 241, 6, 199, 96, 255,
    33, 72, 250, 217, 193, 218, 158, 222, 172, 6, 161, 128, 240, 41, 101, 236, 42, 211, 6, 207,
    118, 106, 213, 48, 65, 128, 40, 162, 21, 101, 171, 33, 125, 98, 188, 110, 233, 56, 53, 175, 15,
    174, 118, 64, 147, 65, 69, 192, 130, 62, 178, 8, 194, 22, 22, 111, 22, 37, 232, 170, 243, 28,
    179, 181, 168, 88, 231, 161, 172, 111, 108, 38, 174, 114, 250, 41, 185, 116, 118, 228, 21, 185,
    109, 247, 33, 85, 200, 139, 2, 129, 129, 0, 190, 13, 155, 248, 7, 18, 2, 126, 151, 116, 134,
    248, 228, 52, 204, 247, 140, 44, 142, 184, 242, 0, 66, 223, 32, 252, 164, 105, 223, 171, 71,
    226, 223, 49, 166, 152, 196, 250, 32, 206, 180, 26, 96, 216, 150, 231, 103, 32, 238, 228, 183,
    187, 38, 179, 241, 220, 80, 216, 226, 222, 31, 117, 71, 245, 172, 127, 49, 211, 215, 207, 83,
    64, 132, 175, 32, 170, 137, 9, 64, 80, 95, 196, 16, 41, 172, 227, 125, 33, 187, 157, 221, 217,
    170, 223, 65, 34, 18, 152, 164, 248, 91, 235, 55, 214, 136, 81, 205, 204, 194, 52, 68, 128,
    202, 91, 160, 236, 85, 159, 162, 112, 110, 218, 225, 10, 227, 194, 51, 185, 2, 129, 128, 90,
    254, 143, 145, 206, 70, 198, 39, 247, 195, 108, 154, 40, 7, 105, 203, 0, 51, 44, 247, 170, 142,
    171, 158, 19, 66, 209, 36, 135, 10, 215, 65, 125, 113, 128, 218, 94, 89, 7, 47, 148, 251, 28,
    90, 243, 168, 95, 85, 216, 115, 139, 237, 62, 170, 202, 239, 7, 161, 231, 45, 141, 124, 159,
    136, 23, 155, 206, 203, 116, 139, 20, 159, 64, 98, 175, 211, 209, 111, 212, 37, 15, 110, 191,
    26, 53, 214, 244, 187, 113, 171, 33, 162, 156, 50, 147, 25, 60, 185, 212, 86, 226, 180, 106,
    216, 176, 171, 211, 195, 174, 222, 84, 36, 40, 60, 187, 184, 239, 210, 183, 80, 88, 72, 64, 27,
    115, 181, 171, 121, 2, 129, 128, 68, 225, 182, 243, 49, 86, 12, 94, 223, 184, 232, 160, 145,
    25, 77, 109, 57, 181, 36, 41, 8, 137, 14, 191, 214, 246, 103, 148, 8, 25, 122, 111, 175, 188,
    133, 19, 5, 174, 191, 77, 35, 51, 92, 77, 116, 124, 30, 20, 130, 169, 3, 185, 116, 208, 247,
    212, 44, 114, 5, 56, 202, 207, 2, 130, 118, 21, 82, 191, 132, 17, 28, 64, 60, 34, 94, 7, 54,
    158, 61, 85, 91, 235, 96, 249, 96, 73, 241, 130, 91, 199, 15, 153, 85, 64, 111, 188, 208, 49,
    5, 131, 149, 194, 92, 216, 21, 193, 221, 147, 88, 46, 8, 30, 98, 157, 9, 225, 216, 139, 110,
    60, 158, 102, 146, 188, 254, 48, 82, 115,
];
const CLIENT: &[u8] = &[
    48, 130, 2, 234, 48, 130, 1, 210, 160, 3, 2, 1, 2, 2, 20, 11, 71, 83, 101, 135, 98, 28, 190,
    209, 105, 101, 19, 33, 184, 27, 238, 190, 217, 81, 240, 48, 13, 6, 9, 42, 134, 72, 134, 247,
    13, 1, 1, 11, 5, 0, 48, 28, 49, 26, 48, 24, 6, 3, 85, 4, 3, 12, 17, 79, 116, 101, 114, 121,
    110, 32, 83, 49, 32, 84, 101, 115, 116, 32, 67, 65, 48, 30, 23, 13, 50, 53, 48, 49, 48, 49, 48,
    48, 48, 48, 48, 48, 90, 23, 13, 51, 53, 48, 49, 48, 49, 48, 48, 48, 48, 48, 48, 90, 48, 27, 49,
    25, 48, 23, 6, 3, 85, 4, 3, 12, 16, 111, 116, 101, 114, 121, 110, 45, 103, 97, 109, 101, 45,
    116, 101, 115, 116, 48, 130, 1, 34, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 3,
    130, 1, 15, 0, 48, 130, 1, 10, 2, 130, 1, 1, 0, 173, 125, 148, 192, 60, 174, 240, 166, 99, 206,
    123, 239, 8, 11, 148, 179, 237, 228, 15, 34, 46, 100, 214, 76, 234, 192, 204, 189, 168, 27,
    203, 2, 22, 225, 155, 239, 55, 247, 57, 110, 68, 226, 106, 206, 161, 46, 224, 138, 154, 157,
    238, 215, 15, 144, 31, 193, 96, 250, 176, 224, 198, 29, 228, 153, 221, 49, 44, 252, 171, 39, 0,
    160, 144, 30, 57, 93, 98, 46, 162, 202, 209, 254, 26, 152, 132, 238, 162, 209, 233, 15, 227,
    233, 125, 68, 219, 34, 24, 55, 115, 12, 69, 51, 99, 96, 177, 217, 218, 137, 58, 143, 33, 206,
    100, 88, 10, 83, 211, 167, 203, 67, 9, 17, 141, 20, 25, 18, 23, 123, 234, 114, 253, 14, 14, 97,
    82, 241, 85, 87, 141, 109, 82, 231, 70, 76, 65, 55, 122, 7, 4, 221, 5, 138, 244, 218, 80, 35,
    127, 199, 15, 95, 238, 110, 191, 40, 73, 9, 132, 42, 2, 54, 80, 5, 183, 236, 226, 220, 62, 54,
    18, 83, 33, 134, 137, 239, 49, 114, 107, 57, 91, 209, 210, 69, 166, 52, 104, 99, 45, 116, 150,
    17, 192, 241, 203, 186, 158, 75, 76, 6, 118, 6, 64, 3, 144, 7, 174, 16, 92, 198, 219, 81, 189,
    115, 155, 222, 218, 127, 82, 50, 239, 251, 37, 229, 222, 61, 254, 144, 100, 173, 159, 233, 228,
    48, 98, 127, 248, 138, 177, 215, 97, 121, 26, 210, 20, 176, 227, 75, 2, 3, 1, 0, 1, 163, 37,
    48, 35, 48, 12, 6, 3, 85, 29, 19, 1, 1, 255, 4, 2, 48, 0, 48, 19, 6, 3, 85, 29, 37, 4, 12, 48,
    10, 6, 8, 43, 6, 1, 5, 5, 7, 3, 2, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 11, 5, 0, 3,
    130, 1, 1, 0, 45, 69, 184, 82, 231, 148, 128, 82, 163, 87, 14, 9, 35, 184, 196, 200, 162, 224,
    46, 6, 224, 200, 37, 235, 237, 137, 129, 0, 128, 69, 125, 194, 64, 133, 14, 72, 124, 215, 152,
    149, 100, 55, 86, 110, 92, 53, 56, 28, 132, 130, 43, 197, 34, 228, 58, 145, 32, 165, 189, 197,
    42, 0, 212, 103, 128, 124, 174, 186, 162, 143, 207, 203, 100, 174, 0, 3, 93, 13, 3, 109, 190,
    205, 47, 38, 204, 34, 236, 42, 149, 188, 70, 164, 170, 43, 194, 198, 67, 56, 44, 111, 1, 251,
    6, 184, 73, 14, 108, 193, 78, 98, 118, 96, 164, 228, 26, 197, 82, 122, 131, 54, 154, 251, 176,
    159, 215, 182, 133, 246, 180, 149, 129, 224, 187, 150, 216, 27, 239, 88, 32, 251, 82, 208, 52,
    65, 156, 246, 251, 177, 161, 152, 197, 12, 254, 228, 105, 73, 141, 111, 5, 250, 96, 139, 18,
    81, 215, 109, 86, 184, 69, 254, 71, 39, 65, 38, 71, 204, 1, 215, 118, 190, 102, 220, 96, 3,
    247, 151, 130, 118, 94, 111, 72, 235, 143, 65, 25, 156, 159, 204, 45, 135, 73, 26, 44, 121, 3,
    132, 174, 170, 150, 134, 198, 50, 12, 112, 41, 223, 66, 137, 20, 89, 19, 68, 4, 105, 7, 58, 24,
    57, 103, 245, 94, 40, 122, 46, 140, 157, 180, 64, 227, 2, 50, 81, 179, 234, 111, 236, 245, 178,
    249, 196, 46, 17, 163, 102, 64, 139,
];
const CLIENT_KEY: &[u8] = &[
    48, 130, 4, 189, 2, 1, 0, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 4, 130, 4,
    167, 48, 130, 4, 163, 2, 1, 0, 2, 130, 1, 1, 0, 173, 125, 148, 192, 60, 174, 240, 166, 99, 206,
    123, 239, 8, 11, 148, 179, 237, 228, 15, 34, 46, 100, 214, 76, 234, 192, 204, 189, 168, 27,
    203, 2, 22, 225, 155, 239, 55, 247, 57, 110, 68, 226, 106, 206, 161, 46, 224, 138, 154, 157,
    238, 215, 15, 144, 31, 193, 96, 250, 176, 224, 198, 29, 228, 153, 221, 49, 44, 252, 171, 39, 0,
    160, 144, 30, 57, 93, 98, 46, 162, 202, 209, 254, 26, 152, 132, 238, 162, 209, 233, 15, 227,
    233, 125, 68, 219, 34, 24, 55, 115, 12, 69, 51, 99, 96, 177, 217, 218, 137, 58, 143, 33, 206,
    100, 88, 10, 83, 211, 167, 203, 67, 9, 17, 141, 20, 25, 18, 23, 123, 234, 114, 253, 14, 14, 97,
    82, 241, 85, 87, 141, 109, 82, 231, 70, 76, 65, 55, 122, 7, 4, 221, 5, 138, 244, 218, 80, 35,
    127, 199, 15, 95, 238, 110, 191, 40, 73, 9, 132, 42, 2, 54, 80, 5, 183, 236, 226, 220, 62, 54,
    18, 83, 33, 134, 137, 239, 49, 114, 107, 57, 91, 209, 210, 69, 166, 52, 104, 99, 45, 116, 150,
    17, 192, 241, 203, 186, 158, 75, 76, 6, 118, 6, 64, 3, 144, 7, 174, 16, 92, 198, 219, 81, 189,
    115, 155, 222, 218, 127, 82, 50, 239, 251, 37, 229, 222, 61, 254, 144, 100, 173, 159, 233, 228,
    48, 98, 127, 248, 138, 177, 215, 97, 121, 26, 210, 20, 176, 227, 75, 2, 3, 1, 0, 1, 2, 130, 1,
    0, 8, 254, 113, 39, 52, 30, 39, 152, 160, 164, 244, 52, 212, 152, 113, 25, 144, 212, 25, 90,
    190, 199, 248, 122, 29, 83, 104, 105, 206, 156, 190, 61, 249, 252, 177, 46, 74, 26, 245, 109,
    187, 129, 119, 99, 5, 229, 29, 187, 225, 191, 29, 66, 37, 234, 229, 82, 139, 154, 3, 112, 107,
    164, 226, 20, 125, 236, 142, 53, 253, 19, 117, 11, 93, 221, 75, 65, 5, 149, 128, 87, 254, 97,
    235, 116, 11, 180, 41, 89, 179, 120, 224, 234, 90, 85, 166, 252, 145, 255, 209, 229, 137, 114,
    220, 12, 135, 187, 209, 1, 251, 56, 175, 90, 121, 70, 61, 38, 210, 90, 169, 5, 109, 35, 109,
    74, 100, 240, 84, 170, 64, 107, 74, 86, 238, 1, 82, 26, 98, 183, 136, 98, 169, 173, 248, 122,
    169, 136, 27, 247, 252, 156, 243, 106, 146, 133, 188, 128, 209, 211, 28, 184, 249, 180, 45, 66,
    29, 52, 173, 10, 69, 155, 132, 22, 169, 78, 107, 230, 81, 166, 150, 73, 106, 215, 34, 91, 142,
    127, 175, 130, 133, 198, 78, 53, 18, 226, 11, 232, 192, 132, 212, 236, 15, 223, 235, 83, 82,
    44, 134, 79, 47, 152, 235, 58, 78, 132, 165, 100, 241, 242, 106, 49, 40, 96, 222, 239, 112, 27,
    167, 134, 210, 132, 140, 247, 204, 253, 100, 30, 14, 172, 43, 101, 94, 76, 217, 2, 121, 46,
    148, 29, 51, 97, 157, 85, 245, 29, 83, 125, 2, 129, 129, 0, 233, 45, 123, 82, 70, 200, 174,
    126, 149, 148, 190, 19, 233, 171, 85, 181, 43, 85, 101, 151, 52, 160, 67, 184, 49, 251, 242,
    82, 80, 85, 118, 240, 138, 201, 254, 155, 121, 16, 174, 119, 57, 88, 16, 66, 36, 94, 150, 86,
    80, 199, 208, 44, 77, 71, 246, 93, 3, 173, 180, 240, 71, 144, 225, 30, 189, 124, 253, 48, 249,
    14, 43, 167, 127, 44, 196, 253, 90, 162, 52, 130, 194, 174, 44, 26, 237, 221, 98, 114, 22, 232,
    190, 231, 25, 91, 35, 7, 254, 234, 228, 203, 212, 208, 154, 106, 163, 15, 120, 250, 89, 74,
    235, 64, 142, 50, 180, 9, 205, 162, 25, 123, 166, 84, 42, 19, 235, 192, 49, 189, 2, 129, 129,
    0, 190, 120, 147, 2, 120, 31, 14, 201, 221, 222, 135, 231, 54, 199, 15, 140, 127, 128, 176,
    199, 160, 139, 184, 160, 255, 89, 220, 198, 224, 27, 23, 52, 62, 210, 232, 22, 45, 98, 49, 74,
    38, 148, 206, 99, 35, 13, 130, 213, 0, 195, 210, 143, 7, 35, 159, 195, 100, 145, 34, 120, 193,
    1, 98, 211, 106, 59, 181, 225, 246, 126, 126, 138, 152, 224, 6, 39, 245, 146, 134, 163, 158,
    61, 0, 183, 28, 60, 40, 233, 181, 178, 101, 214, 214, 109, 74, 189, 98, 230, 140, 85, 226, 169,
    226, 249, 46, 91, 156, 69, 44, 46, 229, 210, 253, 73, 204, 128, 121, 222, 247, 61, 112, 221,
    20, 9, 227, 210, 197, 167, 2, 129, 128, 98, 154, 235, 37, 149, 237, 83, 109, 16, 80, 64, 57,
    39, 125, 94, 215, 40, 38, 10, 185, 240, 117, 61, 109, 237, 37, 64, 9, 243, 18, 209, 35, 83,
    109, 172, 15, 143, 24, 176, 206, 150, 31, 89, 112, 190, 244, 136, 218, 163, 122, 123, 219, 224,
    42, 104, 82, 89, 53, 225, 232, 34, 59, 112, 23, 145, 198, 3, 67, 236, 110, 67, 163, 167, 50, 5,
    115, 166, 26, 37, 222, 141, 249, 29, 137, 157, 193, 26, 218, 104, 57, 176, 81, 5, 245, 246, 80,
    113, 74, 95, 202, 154, 138, 68, 98, 135, 230, 9, 32, 197, 21, 219, 122, 7, 40, 5, 222, 135,
    104, 223, 33, 220, 234, 134, 136, 186, 15, 34, 121, 2, 129, 128, 123, 120, 198, 51, 239, 146,
    146, 143, 161, 216, 67, 217, 74, 213, 55, 235, 40, 44, 80, 108, 216, 82, 58, 217, 131, 46, 201,
    195, 210, 59, 61, 139, 140, 190, 67, 221, 60, 134, 78, 213, 118, 181, 56, 37, 99, 239, 186, 68,
    66, 62, 175, 28, 48, 208, 147, 28, 162, 240, 194, 155, 236, 225, 237, 34, 135, 215, 53, 76,
    201, 241, 174, 46, 66, 115, 215, 9, 200, 83, 184, 28, 16, 212, 18, 198, 160, 243, 14, 194, 146,
    26, 151, 145, 241, 102, 223, 240, 109, 80, 110, 224, 174, 18, 219, 36, 21, 61, 37, 204, 97,
    155, 245, 56, 21, 33, 230, 3, 210, 253, 10, 225, 53, 108, 219, 52, 158, 15, 96, 57, 2, 129,
    129, 0, 196, 26, 100, 113, 65, 160, 65, 147, 173, 214, 185, 211, 144, 94, 252, 13, 13, 81, 1,
    111, 8, 30, 74, 111, 22, 139, 121, 197, 101, 170, 25, 213, 211, 254, 66, 134, 180, 134, 52,
    253, 13, 130, 44, 187, 208, 6, 29, 217, 234, 143, 34, 252, 64, 82, 246, 226, 142, 79, 45, 60,
    245, 94, 122, 212, 213, 137, 60, 137, 95, 244, 117, 208, 108, 175, 191, 218, 151, 188, 62, 74,
    178, 83, 182, 134, 57, 39, 139, 137, 167, 238, 90, 240, 182, 142, 4, 7, 217, 35, 19, 167, 34,
    167, 125, 22, 211, 175, 240, 194, 198, 55, 91, 187, 59, 77, 156, 150, 59, 229, 143, 8, 252,
    166, 148, 104, 83, 124, 118, 216,
];

struct Scripted {
    chunks: VecDeque<Vec<u8>>,
}
impl Scripted {
    fn new(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            chunks: chunks.into(),
        }
    }
}
impl AsyncRead for Scripted {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        dst: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(mut chunk) = self.chunks.pop_front() {
            let n = dst.remaining().min(chunk.len());
            dst.put_slice(&chunk[..n]);
            if n < chunk.len() {
                chunk.drain(..n);
                self.chunks.push_front(chunk)
            }
        }
        Poll::Ready(Ok(()))
    }
}
fn runtime() -> Result<tokio::runtime::Runtime, io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
}
#[test]
fn ingress_is_segmentation_independent_and_increment_atomic()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        for chunks in [
            vec![vec![1; 65_536]],
            vec![vec![1; 32_768], vec![1; 32_768]],
        ] {
            let mut bounded = BoundedHandshakeIo::new(Scripted::new(chunks));
            let mut out = Vec::new();
            bounded.read_to_end(&mut out).await?;
            assert_eq!(out.len(), 65_536);
            assert_eq!(bounded.seen(), 65_536);
        }
        let mut bounded = BoundedHandshakeIo::new(Scripted::new(vec![vec![1; 65_535], vec![2; 2]]));
        let mut out = Vec::new();
        assert!(bounded.read_to_end(&mut out).await.is_err());
        assert_eq!(out.len(), 65_535);
        assert_eq!(bounded.seen(), 65_535);
        bounded.finish_handshake();
        let mut tail = Vec::new();
        bounded.read_to_end(&mut tail).await?;
        assert!(tail.is_empty());
        Ok::<(), io::Error>(())
    })?;
    Ok(())
}
#[test]
fn transient_capacity_is_two_active_and_eight_queued() -> Result<(), Box<dyn std::error::Error>> {
    let capacity = TransientCapacity::new();
    let mut queued = Vec::new();
    for _ in 0..8 {
        queued.push(capacity.try_queue()?)
    }
    assert!(capacity.try_queue().is_err());
    queued[0].try_activate()?;
    queued[1].try_activate()?;
    assert!(queued[2].try_activate().is_err());
    drop(queued.remove(0));
    queued[1].try_activate()?;
    Ok(())
}
fn descriptor(port: u16) -> Result<ProducerDescriptor, Box<dyn std::error::Error>> {
    Ok(ProducerDescriptor::new(
        "platform-test".into(),
        ("127.0.0.1".into(), port),
        "source.test".into(),
        "source.test".into(),
        vec![CertificateDer::from(CA.to_vec())],
        vec![CertificateDer::from(CLIENT.to_vec())],
        PrivateKeyDer::try_from(CLIENT_KEY.to_vec())?,
    )?)
}
#[test]
fn controlled_producer_proves_tls13_mtls_and_four_operations()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let mut roots = RootCertStore::empty();
        roots.add(CertificateDer::from(CA.to_vec()))?;
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots)).build()?;
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![CertificateDer::from(SERVER.to_vec())],
                PrivateKeyDer::try_from(SERVER_KEY.to_vec())?,
            )?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let server = tokio::spawn(async move {
            for operation in [
                Operation::ReadAccountSecurityV1,
                Operation::ReadFreshSigningTrustV1,
                Operation::ReadRecoveryAccountSecurityV2,
                Operation::ReadRecoverySigningTrustV2,
            ] {
                let (tcp, _) = listener.accept().await?;
                let mut tls = acceptor.accept(tcp).await?;
                let mut request = vec![0u8; 4096];
                let n = tls.read(&mut request).await?;
                let text = std::str::from_utf8(&request[..n])
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "utf8"))?;
                if !text.starts_with(&format!("POST {} HTTP/1.1\r\n", operation.path())) {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "path"));
                }
                let (version, name) = match operation {
                    Operation::ReadAccountSecurityV1 => (1, "ReadAccountSecurityV1"),
                    Operation::ReadFreshSigningTrustV1 => (1, "ReadFreshSigningTrustV1"),
                    Operation::ReadRecoveryAccountSecurityV2 => {
                        (2, "ReadRecoveryAccountSecurityV2")
                    }
                    Operation::ReadRecoverySigningTrustV2 => (2, "ReadRecoverySigningTrustV2"),
                    Operation::ReadCharacterBootstrapIntentV1 => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "operation"));
                    }
                };
                let body = format!(
                    "{{\"version\":{version},\"operation\":\"{name}\",\"result\":\"unavailable\"}}"
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                tls.write_all(response.as_bytes()).await?;
                tls.write_all(body.as_bytes()).await?;
            }
            Ok::<(), io::Error>(())
        });
        for operation in [
            Operation::ReadAccountSecurityV1,
            Operation::ReadFreshSigningTrustV1,
            Operation::ReadRecoveryAccountSecurityV2,
            Operation::ReadRecoverySigningTrustV2,
        ] {
            let request = match operation {
                Operation::ReadAccountSecurityV1 => Request::Account {
                    recovery: false,
                    account_id: "01890f47-7c1a-7abc-8def-0123456789ab",
                    purpose: "platform_security",
                    scope: "fresh_admission",
                },
                Operation::ReadFreshSigningTrustV1 => Request::Trust {
                    recovery: false,
                    key_id: "key-1",
                    key_purpose: "fresh_admission",
                },
                Operation::ReadRecoveryAccountSecurityV2 => Request::Account {
                    recovery: true,
                    account_id: "01890f47-7c1a-7abc-8def-0123456789ab",
                    purpose: "platform_security",
                    scope: "existing_actor_recovery",
                },
                Operation::ReadRecoverySigningTrustV2 => Request::Trust {
                    recovery: true,
                    key_id: "key-1",
                    key_purpose: "existing_actor_recovery",
                },
                Operation::ReadCharacterBootstrapIntentV1 => return Err("operation".into()),
            };

            let capacity = TransientCapacity::new();
            let mut permit = capacity.try_queue()?;
            permit.try_activate()?;
            assert_eq!(
                native_admission_source::query(&descriptor(port)?, &request, &mut permit).await?,
                Response::Failure(Failure::Unavailable)
            );
        }
        server.await??;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

#[test]
fn chunked_profile_enforces_count_and_dechunked_body_bounds()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        async fn parse(wire: Vec<u8>) -> Result<Vec<u8>, native_admission_source::SourceError> {
            let (mut writer, mut reader) = tokio::io::duplex(wire.len() + 1);
            writer.write_all(&wire).await?;
            drop(writer);
            native_admission_source::http1_mtls::read_response(&mut reader).await
        }
        let mut accepted = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        for _ in 0..64 {
            accepted.extend_from_slice(b"1\r\nx\r\n");
        }
        accepted.extend_from_slice(b"0\r\n\r\n");
        assert_eq!(parse(accepted).await?.len(), 64);
        let mut too_many = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        for _ in 0..65 {
            too_many.extend_from_slice(b"1\r\nx\r\n");
        }
        too_many.extend_from_slice(b"0\r\n\r\n");
        assert!(parse(too_many).await.is_err());
        let mut too_large =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2001\r\n".to_vec();
        too_large.extend(std::iter::repeat_n(b'x', 8193));
        too_large.extend_from_slice(b"\r\n0\r\n\r\n");
        assert!(parse(too_large).await.is_err());
        for size in [8192, 8193] {
            let mut wire =
                format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n").into_bytes();
            wire.extend(std::iter::repeat_n(b'x', size));
            assert_eq!(parse(wire).await.is_ok(), size == 8192);
        }
        for wire in [
            b"HTTP/1.1 100 Continue\r\nContent-Length: 0\r\n\r\n".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
                .as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Trailer: no\r\n\r\n"
                .as_slice(),
        ] {
            assert!(parse(wire.to_vec()).await.is_err());
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

#[test]
fn descriptor_and_deadline_bounds_are_fixed() -> Result<(), Box<dyn std::error::Error>> {
    use native_admission_source::{
        CONNECT_DEADLINE_MS, EXCHANGE_DEADLINE_MS, HANDSHAKE_DEADLINE_MS,
    };
    assert_eq!(
        (
            CONNECT_DEADLINE_MS,
            HANDSHAKE_DEADLINE_MS,
            EXCHANGE_DEADLINE_MS
        ),
        (1_000, 2_000, 3_000)
    );
    let key = || PrivateKeyDer::try_from(CLIENT_KEY.to_vec());
    assert!(
        ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "other.test".into(),
            vec![CertificateDer::from(CA.to_vec())],
            vec![CertificateDer::from(CLIENT.to_vec())],
            key()?
        )
        .is_err()
    );
    assert!(
        ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![CertificateDer::from(vec![0; 4097])],
            vec![CertificateDer::from(CLIENT.to_vec())],
            key()?
        )
        .is_err()
    );
    assert!(
        ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            (0..17).map(|_| CertificateDer::from(CA.to_vec())).collect(),
            vec![CertificateDer::from(CLIENT.to_vec())],
            key()?
        )
        .is_err()
    );
    assert!(
        ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![CertificateDer::from(CA.to_vec())],
            vec![CertificateDer::from(vec![0; 16])],
            key()?
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn repeated_activation_cannot_leak_capacity() -> Result<(), Box<dyn std::error::Error>> {
    let capacity = TransientCapacity::new();
    let mut permit = capacity.try_queue()?;
    permit.try_activate()?;
    assert!(permit.try_activate().is_err());
    drop(permit);
    let mut permits = Vec::new();
    for _ in 0..8 {
        permits.push(capacity.try_queue()?);
    }
    assert!(capacity.try_queue().is_err());
    Ok(())
}
fn server_config(
    chain_count: usize,
) -> Result<Arc<rustls::ServerConfig>, Box<dyn std::error::Error>> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(CA.to_vec()))?;
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()?;
    let mut chain = vec![CertificateDer::from(SERVER.to_vec())];
    for _ in 1..chain_count {
        chain.push(CertificateDer::from(CA.to_vec()));
    }
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(verifier)
        .with_single_cert(chain, PrivateKeyDer::try_from(SERVER_KEY.to_vec())?)?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}
#[test]
fn authenticated_peer_boundaries_reject_wrong_name_root_client_and_fifth_certificate()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        for case in 0..5 {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let port = listener.local_addr()?.port();
            let acceptor =
                tokio_rustls::TlsAcceptor::from(server_config(if case == 4 { 5 } else { 4 })?);
            let server = tokio::spawn(async move {
                let (tcp, _) = listener.accept().await?;
                if let Ok(mut tls) = acceptor.accept(tcp).await {
                    let mut request = [0u8; 2048];
                    if tls.read(&mut request).await.unwrap_or(0) > 0 {
                        tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                            .await?;
                    }
                }
                Ok::<(), io::Error>(())
            });
            let name = if case == 1 {
                "wrong.test"
            } else {
                "source.test"
            };
            let roots = if case == 2 { SERVER } else { CA };
            let client = if case == 3 { SERVER } else { CLIENT };
            let key = if case == 3 { SERVER_KEY } else { CLIENT_KEY };
            let desc = ProducerDescriptor::new(
                "platform-test".into(),
                ("127.0.0.1".into(), port),
                name.into(),
                name.into(),
                vec![CertificateDer::from(roots.to_vec())],
                vec![CertificateDer::from(client.to_vec())],
                PrivateKeyDer::try_from(key.to_vec())?,
            )?;
            assert_eq!(
                exchange(&desc, Operation::ReadFreshSigningTrustV1, "{}")
                    .await
                    .is_ok(),
                case == 0,
                "case {case}"
            );
            server.await??;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
#[test]
fn stalled_handshake_and_exchange_timeout_and_cancellation_release_socket()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        for handshake in [true, false] {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let port = listener.local_addr()?.port();
            let acceptor = tokio_rustls::TlsAcceptor::from(server_config(1)?);
            let server = tokio::spawn(async move {
                let (mut tcp, _) = listener.accept().await?;
                let mut bytes = [0u8; 4096];
                if handshake {
                    while tcp.read(&mut bytes).await? > 0 {}
                } else {
                    let mut tls = acceptor.accept(tcp).await?;
                    while tls.read(&mut bytes).await.unwrap_or(0) > 0 {}
                }
                Ok::<(), io::Error>(())
            });
            let start = std::time::Instant::now();
            assert!(matches!(
                exchange(&descriptor(port)?, Operation::ReadFreshSigningTrustV1, "{}").await,
                Err(native_admission_source::SourceError::Unavailable)
            ));
            let expected = if handshake { 2000 } else { 3000 };
            assert!(start.elapsed() >= std::time::Duration::from_millis(expected));
            tokio::time::timeout(std::time::Duration::from_secs(2), server).await???;
        }
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let desc = descriptor(listener.local_addr()?.port())?;
        let client =
            tokio::spawn(
                async move { exchange(&desc, Operation::ReadFreshSigningTrustV1, "{}").await },
            );
        let (mut tcp, _) = listener.accept().await?;
        let mut bytes = [0u8; 4096];
        assert!(tcp.read(&mut bytes).await? > 0);
        client.abort();
        assert!(client.await.is_err());
        let eof =
            tokio::time::timeout(std::time::Duration::from_secs(2), tcp.read(&mut bytes)).await??;
        assert_eq!(eof, 0);
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

// The test producer has no publication stage; production callers retain their permit
// through downstream publication/reconciliation, including failed transport outcomes.
async fn exchange(
    desc: &ProducerDescriptor,
    operation: Operation,
    body: &str,
) -> Result<Vec<u8>, native_admission_source::SourceError> {
    let capacity = TransientCapacity::new();
    let mut permit = capacity.try_queue()?;
    permit.try_activate()?;
    bounded_exchange(desc, operation, body, &mut permit).await
}

#[test]
fn expired_queue_and_inactive_exchange_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let capacity = TransientCapacity::new();
        let mut permit = capacity.try_queue()?;
        assert!(
            bounded_exchange(
                &descriptor(1)?,
                Operation::ReadFreshSigningTrustV1,
                "{}",
                &mut permit
            )
            .await
            .is_err()
        );
        tokio::time::sleep(std::time::Duration::from_millis(1001)).await;
        assert!(matches!(
            permit.try_activate(),
            Err(native_admission_source::SourceError::Unavailable)
        ));
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[test]
fn configured_roots_are_four_not_five() -> Result<(), Box<dyn std::error::Error>> {
    for count in [4, 5] {
        let result = ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            (0..count)
                .map(|_| CertificateDer::from(CA.to_vec()))
                .collect(),
            vec![CertificateDer::from(CLIENT.to_vec())],
            PrivateKeyDer::try_from(CLIENT_KEY.to_vec())?,
        );
        assert_eq!(result.is_ok(), count == 4);
    }
    Ok(())
}

#[test]
fn final_chunk_crlf_counts_toward_total_framing() -> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        for over in [false, true] {
            let mut wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
            for i in 0..62 {
                let digits = if i == 0 && !over { 61 } else { 62 };
                wire.extend(std::iter::repeat_n(b'0', digits - 1));
                wire.extend_from_slice(b"1\r\nx\r\n");
            }
            wire.extend_from_slice(b"0\r\n\r\n");
            let (mut writer, mut reader) = tokio::io::duplex(wire.len() + 1);
            writer.write_all(&wire).await?;
            drop(writer);
            assert_eq!(
                native_admission_source::http1_mtls::read_response(&mut reader)
                    .await
                    .is_ok(),
                !over
            );
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[test]
fn descriptor_connect_address_is_fixed_ip_without_dns() -> Result<(), Box<dyn std::error::Error>> {
    for (address, accepted) in [
        ("127.0.0.1", true),
        ("::1", true),
        ("source.test", false),
        ("localhost", false),
    ] {
        let result = ProducerDescriptor::new(
            "platform-test".into(),
            (address.into(), 443),
            "source.test".into(),
            "source.test".into(),
            vec![CertificateDer::from(CA.to_vec())],
            vec![CertificateDer::from(CLIENT.to_vec())],
            PrivateKeyDer::try_from(CLIENT_KEY.to_vec())?,
        );
        assert_eq!(result.is_ok(), accepted, "{address}");
    }
    Ok(())
}
#[test]
fn strict_http_framing_rejects_ambiguous_names_controls_and_signed_lengths()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        for (wire, accepted) in [
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nX-Valid_Token: yes\r\n\r\n{}",
                true,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding : gzip\r\n\r\n{}",
                false,
            ),
            ("HTTP/1.1 200 OK\r\nContent-Length: +2\r\n\r\n{}", false),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n: empty\r\n\r\n{}",
                false,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nX: a\rb\r\n\r\n{}",
                false,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nX: a\nb\r\n\r\n{}",
                false,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nX: a\u{7f}b\r\n\r\n{}",
                false,
            ),
            (
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n+1\r\nx\r\n0\r\n\r\n",
                false,
            ),
            (
                "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n0\r\n\r\n",
                true,
            ),
        ] {
            let (mut writer, mut reader) = tokio::io::duplex(wire.len() + 1);
            writer.write_all(wire.as_bytes()).await?;
            drop(writer);
            assert_eq!(
                native_admission_source::http1_mtls::read_response(&mut reader)
                    .await
                    .is_ok(),
                accepted,
                "{wire:?}"
            );
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[test]
fn character_bootstrap_intent_read_uses_its_separate_endpoint()
-> Result<(), Box<dyn std::error::Error>> {
    const BODY: &str =
        r#"{"contract_version":1,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#;
    const INTENT: &str = r#"{"contract_version":1,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP"}"#;
    assert_eq!(
        Operation::ReadCharacterBootstrapIntentV1.path(),
        "/internal/v1/game-auth/character-bootstrap-intents/read"
    );
    runtime()?.block_on(async {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let acceptor = tokio_rustls::TlsAcceptor::from(server_config(1)?);
        let server = tokio::spawn(async move {
            for status in ["200 OK", "404 Not Found"] {
                let (tcp, _) = listener.accept().await?;
                let mut tls = acceptor.accept(tcp).await?;
                let mut request = vec![0u8; 4096];
                let n = tls.read(&mut request).await?;
                let text = std::str::from_utf8(&request[..n])
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "utf8"))?;
                if !text.starts_with(
                    "POST /internal/v1/game-auth/character-bootstrap-intents/read HTTP/1.1\r\n",
                ) || !text.ends_with(&format!("\r\n\r\n{BODY}"))
                {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "request"));
                }
                let body = if status == "200 OK" { INTENT } else { "" };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                tls.write_all(response.as_bytes()).await?;
            }
            Ok::<(), io::Error>(())
        });
        let issuer = ProducerDescriptor::new(
            native_admission_source::CHARACTER_BOOTSTRAP_INTENT_ISSUER.into(),
            ("127.0.0.1".into(), port),
            "source.test".into(),
            "source.test".into(),
            vec![CertificateDer::from(CA.to_vec())],
            vec![CertificateDer::from(CLIENT.to_vec())],
            PrivateKeyDer::try_from(CLIENT_KEY.to_vec())?,
        )?;
        let capacity = TransientCapacity::new();
        let mut permit = capacity.try_queue()?;
        permit.try_activate()?;
        let raw =
            native_admission_source::read_character_bootstrap_intent(&issuer, BODY, &mut permit)
                .await?;
        assert_eq!(raw, INTENT.as_bytes());
        // Unknown or expired intent is bounded unavailability, not a fallback.
        assert!(matches!(
            native_admission_source::read_character_bootstrap_intent(&issuer, BODY, &mut permit)
                .await,
            Err(native_admission_source::SourceError::Unavailable)
        ));
        server.await??;
        // A descriptor configured for another source cannot read intents.
        assert!(matches!(
            native_admission_source::read_character_bootstrap_intent(
                &descriptor(port)?,
                BODY,
                &mut permit
            )
            .await,
            Err(native_admission_source::SourceError::InvalidDescriptor)
        ));
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

#[test]
fn four_operations_use_the_accepted_platform_endpoint() {
    for operation in [
        Operation::ReadAccountSecurityV1,
        Operation::ReadFreshSigningTrustV1,
        Operation::ReadRecoveryAccountSecurityV2,
        Operation::ReadRecoverySigningTrustV2,
    ] {
        assert_eq!(operation.path(), "/internal/v1/game-auth/native-evidence");
    }
}

#[test]
fn eof_body_and_identity_encoding_follow_bounded_profile() -> Result<(), Box<dyn std::error::Error>>
{
    runtime()?.block_on(async {
        async fn parse(wire: Vec<u8>) -> Result<Vec<u8>, native_admission_source::SourceError> {
            let (mut writer, mut reader) = tokio::io::duplex(wire.len() + 1);
            writer.write_all(&wire).await?;
            drop(writer);
            native_admission_source::http1_mtls::read_response(&mut reader).await
        }
        let body = br#"{"version":1,"operation":"ReadFreshSigningTrustV1","result":"unavailable"}"#;
        for encoding in [
            "",
            "Content-Encoding: identity\r\n",
            "content-encoding: IdEnTiTy\r\n",
        ] {
            for framing in [
                "".to_string(),
                format!("Content-Length: {}\r\n", body.len()),
            ] {
                let mut wire = format!("HTTP/1.1 200 OK\r\n{encoding}{framing}\r\n").into_bytes();
                wire.extend_from_slice(body);
                let raw = parse(wire).await?;
                let request = Request::Trust {
                    recovery: false,
                    key_id: "key-1",
                    key_purpose: "fresh_admission",
                };
                assert_eq!(
                    admission_evidence::decode_response(&request, "platform-test", &raw)
                        .map_err(|_| io::Error::other("decode"))?,
                    Response::Failure(Failure::Unavailable)
                );
            }
        }
        for size in [8192, 8193] {
            let mut wire = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
            wire.extend(std::iter::repeat_n(b'x', size));
            assert_eq!(parse(wire).await.is_ok(), size == 8192);
        }
        for encoding in [
            "Content-Encoding: gzip\r\n",
            "Content-Encoding: identity\r\nContent-Encoding: identity\r\n",
            "Content-Encoding: identity, gzip\r\n",
        ] {
            let wire = format!("HTTP/1.1 200 OK\r\n{encoding}\r\n{{}}").into_bytes();
            assert!(parse(wire).await.is_err());
        }
        let raw = parse(b"HTTP/1.1 200 OK\r\n\r\n{\"version\":1".to_vec()).await?;
        let request = Request::Trust {
            recovery: false,
            key_id: "key-1",
            key_purpose: "fresh_admission",
        };
        assert!(admission_evidence::decode_response(&request, "platform-test", &raw).is_err());
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
#[test]
fn stalled_eof_body_preserves_exchange_deadline() -> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let acceptor = tokio_rustls::TlsAcceptor::from(server_config(1)?);
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut tls = acceptor.accept(tcp).await?;
            let mut buf = [0u8; 2048];
            let request_bytes = tls.read(&mut buf).await?;
            assert!(request_bytes > 0);
            tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Encoding: identity\r\n\r\n{}")
                .await?;
            let _ = tls.read(&mut buf).await;
            Ok::<(), io::Error>(())
        });
        let started = std::time::Instant::now();
        assert!(matches!(
            exchange(&descriptor(port)?, Operation::ReadFreshSigningTrustV1, "{}").await,
            Err(native_admission_source::SourceError::Unavailable)
        ));
        assert!(started.elapsed() >= std::time::Duration::from_millis(3000));
        tokio::time::timeout(std::time::Duration::from_secs(2), server).await???;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
