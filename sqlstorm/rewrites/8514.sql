
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.CreationDate,
        p.ViewCount,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= CURRENT_DATE - INTERVAL '3 years' AND
        p.Score > 0
), 
TopRankedPosts AS (
    SELECT 
        PostId,
        Title,
        Score,
        CreationDate,
        ViewCount,
        OwnerDisplayName
    FROM 
        RankedPosts 
    WHERE 
        Rank <= 5
), 
CommentCounts AS (
    SELECT 
        PostId,
        COUNT(*) AS TotalComments
    FROM 
        Comments
    GROUP BY 
        PostId
)
SELECT 
    trp.Title,
    trp.Score,
    trp.CreationDate,
    trp.ViewCount,
    trp.OwnerDisplayName,
    COALESCE(cc.TotalComments, 0) AS CommentCount
FROM 
    TopRankedPosts trp
LEFT JOIN 
    CommentCounts cc ON trp.PostId = cc.PostId
ORDER BY 
    trp.Score DESC, trp.ViewCount DESC;
