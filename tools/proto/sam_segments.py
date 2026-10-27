"""Run SAM (segment-anything, Apache-2.0) automatic masks on one image; separate venv.

Usage: .venv-sam/bin/python tools/proto/sam_segments.py IMAGE.png CHECKPOINT OUT.npz
Writes an int32 label image (0 = no mask; smaller masks drawn over larger ones) and areas.
"""
import sys

import numpy as np
import torch
from PIL import Image
from segment_anything import SamAutomaticMaskGenerator, sam_model_registry


def main():
    image_path, checkpoint, out = sys.argv[1:4]
    torch.set_num_threads(16)
    image = np.asarray(Image.open(image_path).convert("RGB"))
    sam = sam_model_registry["vit_b"](checkpoint=checkpoint)
    generator = SamAutomaticMaskGenerator(sam, points_per_side=32, pred_iou_thresh=0.86,
                                          stability_score_thresh=0.9, min_mask_region_area=200)
    masks = sorted(generator.generate(image), key=lambda m: m["area"], reverse=True)
    labels = np.zeros(image.shape[:2], np.int32)
    for index, mask in enumerate(masks, start=1):
        labels[mask["segmentation"]] = index
    np.savez_compressed(out, labels=labels, areas=np.array([m["area"] for m in masks]),
                        scores=np.array([m["predicted_iou"] for m in masks]))
    print(f"{len(masks)} masks; unlabeled fraction {float((labels == 0).mean()):.3f}")


if __name__ == "__main__":
    main()
