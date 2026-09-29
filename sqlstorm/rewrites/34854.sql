
WITH RecursivePostCTE AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.AnswerCount,
        p.ViewCount,
        p.CreationDate,
        p.Score,
        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(SUM(v.BountyAmount), 0) DESC) AS PostRank
    FROM 
        Posts p
    LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) 
    GROUP BY 
        p.Id, p.Title, p.AnswerCount, p.ViewCount, p.CreationDate, p.Score, p.OwnerUserId
),
FilteredPosts AS (
    SELECT 
        *,
        CASE 
            WHEN Score > 10 THEN 'High Score'
            WHEN Score BETWEEN 5 AND 10 THEN 'Medium Score'
            ELSE 'Low Score'
        END AS ScoreCategory
    FROM 
        RecursivePostCTE
    WHERE 
        CreationDate > DATE '2024-10-01' - INTERVAL '1 year'
),
PostComments AS (
    SELECT 
        c.PostId,
        COUNT(c.Id) AS CommentCount
    FROM 
        Comments c
    GROUP BY 
        c.PostId
),
FinalResults AS (
    SELECT 
        fp.PostId,
        fp.Title,
        fp.ViewCount,
        fp.AnswerCount,
        fp.TotalBounty,
        fp.Score,
        fp.ScoreCategory,
        COALESCE(pc.CommentCount, 0) AS CommentCount
    FROM 
        FilteredPosts fp
    LEFT JOIN PostComments pc ON fp.PostId = pc.PostId
    WHERE 
        fp.ScoreCategory <> 'Low Score'
)
SELECT 
    fr.Title,
    fr.ViewCount,
    fr.AnswerCount,
    fr.TotalBounty,
    fr.Score,
    fr.ScoreCategory,
    fr.CommentCount,
    ROW_NUMBER() OVER (ORDER BY fr.TotalBounty DESC, fr.Score DESC, fr.PostId) AS FinalRank
FROM 
    FinalResults fr
WHERE 
    fr.CommentCount > 5
ORDER BY 
    fr.Score ASC, fr.ViewCount DESC;
