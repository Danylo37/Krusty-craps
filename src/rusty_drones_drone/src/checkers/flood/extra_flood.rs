#![cfg(test)]
use crate::checkers::flood::assert_topology_of_drones;
use crate::checkers::TIMEOUT;

#[test]
fn test_matrix_loop_flood() {
    assert_topology_of_drones(
        19,
        &[
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (6, 8),
            (8, 9),
            (10, 1),
            (11, 2),
            (12, 3),
            (13, 4),
            (14, 5),
            (15, 6),
            (16, 7),
            (17, 8),
            (18, 9),
            (10, 11),
            (11, 12),
            (12, 13),
            (13, 14),
            (14, 15),
            (15, 16),
            (16, 17),
            (17, 18),
        ],
        TIMEOUT,
    );
}

#[test]
fn test_star_loop_flood() {
    assert_topology_of_drones(
        11,
        &[
            (0, 1),
            (1, 4),
            (2, 5),
            (3, 6),
            (4, 7),
            (5, 8),
            (6, 9),
            (7, 10),
            (8, 1),
            (9, 2),
            (10, 3),
        ],
        TIMEOUT,
    );
}

#[test]
fn test_butterfly_loop_flood() {
    assert_topology_of_drones(
        11,
        &[
            (0, 1),
            (1, 5),
            (1, 6),
            (2, 5),
            (2, 6),
            (3, 7),
            (3, 8),
            (4, 7),
            (4, 8),
            (6, 10),
            (7, 9),
            (9, 10),
            (5, 9),
            (8, 10),
        ],
        TIMEOUT,
    );
}

#[test]
fn test_tree_loop_flood() {
    assert_topology_of_drones(
        11,
        &[
            (0, 1),
            (1, 2),
            (1, 3),
            (2, 4),
            (2, 5),
            (2, 6),
            (3, 4),
            (3, 5),
            (3, 6),
            (4, 7),
            (4, 8),
            (4, 9),
            (4, 10),
            (5, 7),
            (5, 8),
            (5, 9),
            (5, 10),
            (6, 7),
            (6, 8),
            (6, 9),
            (6, 10),
        ],
        TIMEOUT,
    );
}
