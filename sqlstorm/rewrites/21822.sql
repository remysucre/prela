
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id, c.Id) AS Rank,
        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
)

SELECT 
    rp.PostId,
    rp.Title,
    rp.CreationDate,
    rp.Score,
    rp.ViewCount,
    rp.Rank,
    rp.CommentCount,
    COALESCE(b.Name, 'No Badge') AS BadgeName,
    CASE 
        WHEN rp.ViewCount IS NULL THEN 'No Views'
        WHEN rp.ViewCount > 100 THEN 'High View Count'
        ELSE 'Moderate View Count'
    END AS ViewStatus,
    STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
FROM 
    RankedPosts rp
LEFT JOIN 
    Users u ON rp.PostId = u.Id
LEFT JOIN 
    Badges b ON u.Id = b.UserId AND b.Date = (SELECT MAX(b2.Date) FROM Badges b2 WHERE b2.UserId = u.Id)
LEFT JOIN 
    LATERAL (
        SELECT 
            unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS TagName
        FROM 
            Posts p
        WHERE 
            p.Id = rp.PostId
    ) t ON TRUE
WHERE 
    rp.Rank <= 5
    AND (rp.Score > 10 OR rp.ViewCount IS NOT NULL)
GROUP BY 
    rp.PostId,
    rp.Title,
    rp.CreationDate,
    rp.Score,
    rp.ViewCount,
    rp.Rank,
    rp.CommentCount,
    b.Name
ORDER BY 
    rp.CreationDate DESC;
