
WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        p.AnswerCount,
        u.DisplayName AS OwnerDisplayName,
        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' 
        AND p.PostTypeId = 1 
),
TaggedPosts AS (
    SELECT 
        rp.Id,
        rp.Title,
        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags,
        rp.OwnerDisplayName,
        rp.CreationDate,
        rp.Score,
        rp.ViewCount,
        rp.AnswerCount,
        rp.RankScore
    FROM 
        RankedPosts rp
    JOIN 
        LATERAL (SELECT unnest(string_to_array(rp.Title, ' ')) AS TagName) t ON t.TagName IS NOT NULL
    GROUP BY 
        rp.Id, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.RankScore
)
SELECT 
    tp.Title,
    tp.OwnerDisplayName,
    tp.CreationDate,
    tp.Score,
    tp.ViewCount,
    tp.AnswerCount,
    tp.Tags
FROM 
    TaggedPosts tp
WHERE 
    tp.RankScore <= 10
ORDER BY 
    tp.Score DESC, tp.CreationDate DESC;
