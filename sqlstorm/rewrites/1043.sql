
WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.Score,
        p.AnswerCount,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn,
        p.OwnerUserId,
        p.Tags
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1
),
UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.Reputation
),
PopularTags AS (
    SELECT 
        TRIM(unnest(string_to_array(Tags, '><'))) AS Tag
    FROM 
        Posts
    WHERE 
        PostTypeId = 1
),
TagStatistics AS (
    SELECT 
        t.Tag AS TagName,
        COUNT(*) AS TagCount
    FROM 
        PopularTags t
    GROUP BY 
        t.Tag
    ORDER BY 
        TagCount DESC
    LIMIT 10
)
SELECT 
    up.UserId,
    up.Reputation,
    up.TotalBounty,
    rp.Title,
    rp.Score,
    rp.AnswerCount,
    rp.ViewCount,
    tt.TagName
FROM 
    UserReputation up
JOIN 
    RankedPosts rp ON up.UserId = rp.OwnerUserId
LEFT JOIN 
    TagStatistics tt ON tt.TagName = rp.Tags
WHERE 
    up.Reputation > 1000
    AND (rp.AnswerCount > 5 OR rp.ViewCount > 100)
ORDER BY 
    up.Reputation DESC, rp.Score DESC, rp.Id
LIMIT 50;
