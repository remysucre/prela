
WITH PostTagCount AS (
    SELECT 
        p.Id AS PostId,
        COUNT(DISTINCT t.TagName) AS TagCount,
        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
    FROM 
        Posts p 
    INNER JOIN 
        UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS t(TagName) ON TRUE
    WHERE 
        p.PostTypeId = 1 
    GROUP BY 
        p.Id
),
PopularUsers AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        SUM(p.ViewCount) AS TotalViews,
        COUNT(DISTINCT p.Id) AS QuestionsAnswered
    FROM 
        Users u
    JOIN 
        Posts p ON u.Id = p.OwnerUserId
    WHERE 
        p.PostTypeId IN (2, 1) 
    GROUP BY 
        u.Id, u.DisplayName
    HAVING 
        COUNT(DISTINCT p.Id) > 5
),
RecentPostActivity AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        COALESCE(ph.Comment, 'No comments') AS RecentComment,
        RANK() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS CommentRank
    FROM 
        Posts p
    LEFT JOIN 
        PostHistory ph ON p.Id = ph.PostId
    WHERE 
        p.PostTypeId IN (1, 2)
)
SELECT 
    pt.PostId,
    ra.Title,
    pt.TagCount,
    pt.Tags,
    pu.DisplayName AS ActiveUser,
    pu.TotalViews,
    ra.RecentComment
FROM 
    PostTagCount pt
JOIN 
    RecentPostActivity ra ON pt.PostId = ra.PostId
JOIN 
    PopularUsers pu ON pu.QuestionsAnswered > 5 AND pu.UserId = ra.PostId
WHERE 
    ra.CommentRank = 1
ORDER BY 
    pt.TagCount DESC, pu.TotalViews DESC;
